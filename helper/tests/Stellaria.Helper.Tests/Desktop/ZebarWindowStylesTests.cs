using Stellaria.Helper.Desktop;
using Stellaria.Helper.Interop;

namespace Stellaria.Helper.Tests.Desktop;

public sealed class ZebarWindowStylesTests
{
    [Theory]
    [InlineData("zebar", "Tauri Window", "Zebar - stellaria / dashboard", true)]
    [InlineData("ZEBAR", "Tauri Window", "Zebar - stellaria / popout", true)]
    [InlineData("zebar", "Shell_TrayWnd", "Shell_TrayWnd", false)]
    [InlineData("zebar", "Tauri Window", "Tauri App", false)]
    [InlineData("other", "Tauri Window", "Zebar - stellaria / dashboard", false)]
    public void Only_stellaria_widget_windows_are_repaired(string process, string windowClass, string title, bool expected)
    {
        Assert.Equal(expected, ZebarWindowStyles.IsWidget(process, windowClass, title));
    }

    [Fact]
    public void Corrected_style_is_a_tool_window_and_not_an_app_window()
    {
        var original = User32.WsExAppWindow | User32.WsExNoActivate;

        var corrected = ZebarWindowStyles.CorrectedExtendedStyle(original);

        Assert.NotEqual(0, corrected & User32.WsExToolWindow);
        Assert.Equal(0, corrected & User32.WsExAppWindow);
        Assert.NotEqual(0, corrected & User32.WsExNoActivate);
    }

    [Fact]
    public void Frame_refresh_flags_preserve_widget_geometry()
    {
        Assert.NotEqual(0u, ZebarWindowStyles.RefreshFlags & User32.SwpNoSize);
        Assert.NotEqual(0u, ZebarWindowStyles.RefreshFlags & User32.SwpNoMove);
    }
}
