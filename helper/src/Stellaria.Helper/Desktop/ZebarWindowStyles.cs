using Stellaria.Helper.Interop;

namespace Stellaria.Helper.Desktop;

internal static class ZebarWindowStyles
{
    public const string WidgetTitlePrefix = "Zebar - stellaria / ";
    public const string WidgetWindowClass = "Tauri Window";

    public static bool IsWidget(string processName, string className, string title) =>
        processName.Equals("zebar", StringComparison.OrdinalIgnoreCase) &&
        className.Equals(WidgetWindowClass, StringComparison.Ordinal) &&
        title.StartsWith(WidgetTitlePrefix, StringComparison.Ordinal);

    public static int CorrectedExtendedStyle(int style) =>
        (style | User32.WsExToolWindow) & ~User32.WsExAppWindow;

    public static int Repair(ProcessDirectory processes)
    {
        var repaired = 0;
        foreach (var hwnd in User32.TopLevelWindows())
        {
            var processName = processes.NameOf(User32.ProcessId(hwnd));
            if (!IsWidget(processName, User32.ClassName(hwnd), User32.WindowText(hwnd)))
            {
                continue;
            }

            var current = User32.GetWindowLong(hwnd, User32.GwlExStyle);
            var corrected = CorrectedExtendedStyle(current);
            if (current == corrected)
            {
                continue;
            }

            User32.SetWindowLong(hwnd, User32.GwlExStyle, corrected);
            User32.SetWindowPos(hwnd, 0, 0, 0, 0, 0,
                User32.SwpNoZOrder | User32.SwpNoActivate | User32.SwpFrameChanged);
            repaired++;
        }

        return repaired;
    }
}
