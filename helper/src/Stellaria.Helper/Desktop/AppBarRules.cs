using Stellaria.Helper.Interop;

namespace Stellaria.Helper.Desktop;

internal enum DockEdge : uint
{
    Left = 0,
    Top = 1,
    Right = 2,
    Bottom = 3,
}

internal sealed record Dock(nint Handle, Rect Bounds, DockEdge Edge, int Thickness);

internal static class AppBarRules
{
    public const int MaxThickness = 64;

    public static readonly string[] DockTitleSuffixes = ["stellaria / bar", "stellaria / edge"];

    public static bool IsDockTitle(string title) => DockTitleSuffixes.Any(suffix => title.EndsWith(suffix, StringComparison.Ordinal));

    public static Dock? Classify(nint handle, Rect window, Rect monitor)
    {
        var horizontal = window.Width > window.Height;
        var thickness = horizontal ? window.Height : window.Width;
        if (thickness <= 0 || thickness > MaxThickness)
        {
            return null;
        }

        var edge = horizontal
            ? window.Top + window.Bottom < monitor.Top + monitor.Bottom ? DockEdge.Top : DockEdge.Bottom
            : window.Left + window.Right < monitor.Left + monitor.Right ? DockEdge.Left : DockEdge.Right;
        return new Dock(handle, window, edge, thickness);
    }

    public static bool TouchesItsEdge(Dock dock, Rect monitor) => dock.Edge switch
    {
        DockEdge.Left => dock.Bounds.Left == monitor.Left,
        DockEdge.Top => dock.Bounds.Top == monitor.Top,
        DockEdge.Right => dock.Bounds.Right == monitor.Right,
        _ => dock.Bounds.Bottom == monitor.Bottom,
    };

    public static Rect SnapToEdge(Dock dock, Rect monitor) => dock.Edge switch
    {
        DockEdge.Left => new Rect(monitor.Left, monitor.Top, monitor.Left + dock.Thickness, monitor.Bottom),
        DockEdge.Top => new Rect(monitor.Left, monitor.Top, monitor.Right, monitor.Top + dock.Thickness),
        DockEdge.Right => new Rect(monitor.Right - dock.Thickness, monitor.Top, monitor.Right, monitor.Bottom),
        _ => new Rect(monitor.Left, monitor.Bottom - dock.Thickness, monitor.Right, monitor.Bottom),
    };

    public static bool IsReserved(Dock dock, Rect work) => dock.Edge switch
    {
        DockEdge.Left => work.Left >= dock.Bounds.Right,
        DockEdge.Top => work.Top >= dock.Bounds.Bottom,
        DockEdge.Right => work.Right <= dock.Bounds.Left,
        _ => work.Bottom <= dock.Bounds.Top,
    };
}

internal sealed class RepairBreaker(int maxRepairs, TimeSpan window, TimeSpan pause, TimeProvider time)
{
    private readonly Queue<DateTimeOffset> _repairs = new();
    private DateTimeOffset _pausedUntil = DateTimeOffset.MinValue;

    public bool IsPaused => time.GetUtcNow() < _pausedUntil;

    public bool RecordRepair()
    {
        var now = time.GetUtcNow();
        while (_repairs.Count > 0 && now - _repairs.Peek() >= window)
        {
            _repairs.Dequeue();
        }

        _repairs.Enqueue(now);
        if (_repairs.Count < maxRepairs)
        {
            return false;
        }

        _repairs.Clear();
        _pausedUntil = now + pause;
        return true;
    }
}
