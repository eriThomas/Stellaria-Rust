using System.Text.Json.Nodes;
using Stellaria.Helper.Desktop;
using Stellaria.Helper.Hosting;
using Stellaria.Helper.Interop;

namespace Stellaria.Helper.Modules;

internal sealed class ShellModule(HelperState state) : IHelperModule
{
    public static readonly TimeSpan MaintenanceInterval = TimeSpan.FromSeconds(2);
    public static readonly TimeSpan ResumeDelay = TimeSpan.FromSeconds(4);

    public static readonly TimeSpan PushedLimit = TimeSpan.FromSeconds(6);

    private DateTimeOffset? _pushedSince;

    private ModuleContext _context = null!;
    private RepairBreaker _breaker = null!;
    private Timer? _maintenance;
    private bool _hideTaskbar;

    public string Name => "shell";

    public Task StartAsync(ModuleContext context)
    {
        _context = context;
        _breaker = new RepairBreaker(5, TimeSpan.FromMinutes(1), TimeSpan.FromMinutes(5), context.Time);
        context.Config.Changed += _ =>
        {
            var hidden = context.Config.GetFlag("taskbar.hide");
            context.Loop.Post(() =>
            {
                if (hidden != _hideTaskbar)
                {
                    ApplyTaskbar(hidden);
                }
            });
        };
        context.Loop.SessionEnding += () => ShellRecovery.ShowTaskbar(context.Log);
        context.Loop.Resumed += () => _ = RestartAfterResumeAsync();
        _maintenance = new Timer(_ => context.Loop.Post(Maintain), null, MaintenanceInterval, MaintenanceInterval);
        context.Shutdown.Register(() => _maintenance.Dispose());
        return context.Loop.InvokeAsync(() => ApplyTaskbar(context.Config.GetFlag("taskbar.hide")));
    }

    private async Task RestartAfterResumeAsync()
    {
        _context.Log.Info("shell", "The PC woke up from sleep; Zebar is restarted so its windows draw again.");
        try
        {
            await Task.Delay(ResumeDelay, _context.Shutdown).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            return;
        }

        ZebarWatchdog.TryRestartRunning(_context.Log, _context.Desktop.Processes, _context.Time.GetUtcNow(), "The PC woke up from sleep", ignoreCooldown: true);
    }

    private void ApplyTaskbar(bool hidden)
    {
        _hideTaskbar = hidden;
        if (hidden)
        {
            state.OriginalTaskbarAutoHide ??= Taskbar.AutoHide;
            Taskbar.SetAutoHide(true);
            Taskbar.SetVisible(false);
        }
        else
        {
            ShellRecovery.RestoreOriginalTaskbar(state, _context.Log);
        }

        PublishState();
    }

    private void PublishState() => _context.Hub.Publish("shell", new JsonObject { ["taskbarHidden"] = _hideTaskbar });

    private void Maintain()
    {
        if (_hideTaskbar)
        {
            Taskbar.SetAutoHide(true);
            Taskbar.SetVisible(false);
        }

        if (_breaker.IsPaused)
        {
            return;
        }

        var repaired = RepairAppBars(out var pushedZebar);
        HandlePushedDocks(pushedZebar);
        if (repaired == 0)
        {
            return;
        }

        _context.Log.Info("shell", $"Re-registered {repaired} Stellaria app bar(s).");
        _context.Hub.Raise("appbars.repaired", new JsonObject { ["count"] = repaired });
        if (_breaker.RecordRepair())
        {
            _context.Log.Warning("shell", "App bar reservations keep getting lost, repairs paused for 5 minutes.");
            _context.Hub.Raise("appbars.paused", new JsonObject { ["minutes"] = 5 });
        }
    }

    public static bool ShouldRestartForPushedDocks(DateTimeOffset? pushedSince, DateTimeOffset now) =>
        pushedSince is { } since && now - since >= PushedLimit;

    private void HandlePushedDocks(uint pushedZebar)
    {
        var now = _context.Time.GetUtcNow();
        if (pushedZebar == 0)
        {
            if (_pushedSince is not null)
            {
                _context.Log.Info("shell", "Stellaria docks are back against their screen edges.");
            }

            _pushedSince = null;
            return;
        }

        if (_pushedSince is null)
        {
            _pushedSince = now;
            _context.Log.Warning("shell", "Stellaria docks were pushed away from their screen edges (display change?); Zebar is restarted if this lasts.");
            return;
        }

        if (ShouldRestartForPushedDocks(_pushedSince, now) &&
            ZebarWatchdog.TryRestart(_context.Log, _context.Desktop.Processes, pushedZebar, now, "Stellaria docks stayed away from their screen edges"))
        {
            _pushedSince = null;
        }
    }

    private unsafe int RepairAppBars(out uint pushedZebar)
    {
        pushedZebar = 0;
        var repaired = 0;
        foreach (var hwnd in User32.TopLevelWindows())
        {
            if (!User32.IsWindowVisible(hwnd) || !AppBarRules.IsDockTitle(User32.WindowText(hwnd)))
            {
                continue;
            }

            if (!string.Equals(_context.Desktop.Processes.NameOf(User32.ProcessId(hwnd)), "zebar", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var info = MonitorInfoEx.Create();
            if (!User32.GetWindowRect(hwnd, out var bounds) || !User32.GetMonitorInfo(User32.MonitorFromWindow(hwnd, User32.MonitorDefaultToNearest), ref info))
            {
                continue;
            }

            if (AppBarRules.Classify(hwnd, bounds, info.Monitor) is not { } dock || AppBarRules.IsReserved(dock, info.Work))
            {
                continue;
            }

            if (!AppBarRules.TouchesItsEdge(dock, info.Monitor))
            {
                var corrected = AppBarRules.SnapToEdge(dock, info.Monitor);
                if (!User32.SetWindowPos(
                        hwnd,
                        0,
                        corrected.Left,
                        corrected.Top,
                        corrected.Width,
                        corrected.Height,
                        User32.SwpNoZOrder | User32.SwpNoActivate))
                {
                    pushedZebar = User32.ProcessId(hwnd);
                    continue;
                }

                dock = dock with { Bounds = corrected };
                _context.Log.Info("shell", $"Moved a displaced Stellaria dock back to the {dock.Edge.ToString().ToLowerInvariant()} edge.");
            }

            var data = new AppBarData { Size = sizeof(AppBarData), Hwnd = hwnd, Edge = (uint)dock.Edge, Rect = dock.Bounds };
            Shell32.SHAppBarMessage(Shell32.AbmNew, ref data);
            data.Rect = dock.Bounds;
            Shell32.SHAppBarMessage(Shell32.AbmSetPos, ref data);
            repaired++;
        }

        return repaired;
    }
}
