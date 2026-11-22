using System.Text.Json.Nodes;
using Stellaria.Helper.Desktop;
using Stellaria.Helper.Server;

namespace Stellaria.Helper.Modules;

internal sealed class HealthModule : IHelperModule
{
    public static readonly TimeSpan CheckInterval = TimeSpan.FromSeconds(5);
    public static readonly TimeSpan StallThreshold = TimeSpan.FromSeconds(10);

    private readonly HashSet<ClientSession> _stalled = [];
    private Timer? _check;
    private ZebarWatchdog? _watchdog;

    public string Name => "health";

    public Task StartAsync(ModuleContext context)
    {
        context.Router.Register("widgets.reload", (request, _) =>
        {
            context.Log.Info("health", $"Reload of every widget requested by {request.Widget}.");
            context.Hub.Raise("widgets.reload", new JsonObject());
            return Task.FromResult<JsonNode?>(null);
        });
        _watchdog = new ZebarWatchdog(context.Log, context.Desktop.Processes);
        _check = new Timer(_ => Check(context), null, CheckInterval, CheckInterval);
        context.Shutdown.Register(() => _check.Dispose());
        return Task.CompletedTask;
    }

    public static bool IsStalled(bool authenticated, bool reportsVisible, DateTimeOffset lastHeartbeat, DateTimeOffset now) =>
        authenticated && reportsVisible && now - lastHeartbeat > StallThreshold;

    private void Check(ModuleContext context)
    {
        var now = context.Time.GetUtcNow();
        _watchdog?.Check(now);
        var sessions = context.Hub.Sessions;
        lock (_stalled)
        {
            _stalled.RemoveWhere(session => !sessions.Contains(session));
            foreach (var session in sessions)
            {
                var stalled = IsStalled(session.IsAuthenticated, session.ReportsVisible, session.LastHeartbeat, now);
                if (stalled && _stalled.Add(session))
                {
                    context.Log.Warning("health", $"{session.Describe()} is visible but stopped rendering frames {(now - session.LastHeartbeat).TotalSeconds:F0} s ago.");
                }
                else if (!stalled && _stalled.Remove(session))
                {
                    context.Log.Info("health", $"{session.Describe()} is rendering again.");
                }
            }
        }
    }
}
