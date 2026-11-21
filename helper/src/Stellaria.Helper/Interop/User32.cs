using System.Runtime.InteropServices;

namespace Stellaria.Helper.Interop;

internal static unsafe partial class User32
{
    public const int GwlExStyle = -20;
    public const int WsExTransparent = 0x20;
    public const int WsExToolWindow = 0x80;
    public const int WsExAppWindow = 0x40000;
    public const int WsExNoActivate = 0x08000000;
    public const uint GwOwner = 4;
    public const uint MonitorDefaultToNearest = 2;
    public const int SwHide = 0;
    public const int SwShowNormal = 1;
    public const int SwShow = 5;
    public const int SwMinimize = 6;
    public const int SwMaximize = 3;
    public const int SwRestore = 9;
    public const uint SwpNoSize = 0x0001;
    public const uint SwpNoMove = 0x0002;
    public const uint SwpNoZOrder = 0x0004;
    public const uint SwpNoActivate = 0x0010;
    public const uint SwpFrameChanged = 0x0020;
    public const uint WmClose = 0x0010;
    public const uint WmQueryEndSession = 0x0011;
    public const uint WmEndSession = 0x0016;
    public const uint WmPowerBroadcast = 0x0218;
    public const nuint PbtResumeAutomatic = 0x0012;
    public const uint WmSettingChange = 0x001A;
    public const uint WmDisplayChange = 0x007E;
    public const uint WmInputLangChangeRequest = 0x0050;
    public const uint WmHotkey = 0x0312;
    public const uint WmApp = 0x8000;
    public const uint WmQuit = 0x0012;
    public const uint ModNoRepeat = 0x4000;
    public const uint KeyEventKeyUp = 0x0002;

    [LibraryImport("user32.dll")]
    public static partial nint GetForegroundWindow();

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SetForegroundWindow(nint hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool AllowSetForegroundWindow(int processId);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool BringWindowToTop(nint hwnd);

    [LibraryImport("user32.dll", EntryPoint = "GetWindowTextW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial int GetWindowText(nint hwnd, char* text, int max);

    [LibraryImport("user32.dll", EntryPoint = "GetWindowTextLengthW")]
    public static partial int GetWindowTextLength(nint hwnd);

    [LibraryImport("user32.dll", EntryPoint = "GetClassNameW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial int GetClassName(nint hwnd, char* text, int max);

    [LibraryImport("user32.dll")]
    public static partial uint GetWindowThreadProcessId(nint hwnd, out uint processId);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool GetWindowRect(nint hwnd, out Rect rect);

    [LibraryImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SetWindowPos(nint hwnd, nint insertAfter, int x, int y, int width, int height, uint flags);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsWindow(nint hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsWindowVisible(nint hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsIconic(nint hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsZoomed(nint hwnd);

    [LibraryImport("user32.dll", EntryPoint = "GetWindowLongW")]
    public static partial int GetWindowLong(nint hwnd, int index);

    [LibraryImport("user32.dll", EntryPoint = "SetWindowLongW", SetLastError = true)]
    public static partial int SetWindowLong(nint hwnd, int index, int value);

    [LibraryImport("user32.dll")]
    public static partial nint GetWindow(nint hwnd, uint command);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool ShowWindow(nint hwnd, int command);

    [LibraryImport("user32.dll", EntryPoint = "PostMessageW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool PostMessage(nint hwnd, uint message, nuint wParam, nint lParam);

    [LibraryImport("user32.dll", EntryPoint = "PostThreadMessageW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool PostThreadMessage(uint threadId, uint message, nuint wParam, nint lParam);

    [LibraryImport("user32.dll", EntryPoint = "FindWindowW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial nint FindWindow(string? className, string? windowName);

    [LibraryImport("user32.dll", EntryPoint = "FindWindowExW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial nint FindWindowEx(nint parent, nint childAfter, string? className, string? windowName);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool EnumWindows(delegate* unmanaged<nint, nint, int> callback, nint data);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool EnumChildWindows(nint parent, delegate* unmanaged<nint, nint, int> callback, nint data);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool EnumDisplayMonitors(nint hdc, nint clip, delegate* unmanaged<nint, nint, Rect*, nint, int> callback, nint data);

    [LibraryImport("user32.dll", EntryPoint = "GetMonitorInfoW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool GetMonitorInfo(nint monitor, ref MonitorInfoEx info);

    [LibraryImport("user32.dll")]
    public static partial nint MonitorFromWindow(nint hwnd, uint flags);

    [LibraryImport("user32.dll")]
    public static partial nint MonitorFromPoint(Point point, uint flags);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool GetCursorPos(out Point point);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool AttachThreadInput(uint attach, uint attachTo, [MarshalAs(UnmanagedType.Bool)] bool doAttach);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool RegisterHotKey(nint hwnd, int id, uint modifiers, uint virtualKey);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool UnregisterHotKey(nint hwnd, int id);

    [LibraryImport("user32.dll")]
    public static partial short GetKeyState(int virtualKey);

    [LibraryImport("user32.dll")]
    public static partial void keybd_event(byte virtualKey, byte scan, uint flags, nuint extraInfo);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool GetLastInputInfo(ref LastInputInfo info);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool LockWorkStation();

    [LibraryImport("user32.dll")]
    public static partial nint GetKeyboardLayout(uint threadId);

    [LibraryImport("user32.dll")]
    public static partial int GetKeyboardLayoutList(int count, nint* list);

    [LibraryImport("user32.dll", EntryPoint = "SystemParametersInfoW", StringMarshalling = StringMarshalling.Utf16)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SystemParametersInfo(uint action, uint parameter, string? value, uint winIni);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool PrintWindow(nint hwnd, nint hdc, uint flags);

    [LibraryImport("user32.dll", EntryPoint = "RegisterClassExW", SetLastError = true)]
    public static partial ushort RegisterClassEx(WndClassEx* windowClass);

    [LibraryImport("user32.dll", EntryPoint = "CreateWindowExW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial nint CreateWindowEx(uint exStyle, string className, string windowName, uint style, int x, int y, int width, int height, nint parent, nint menu, nint instance, nint param);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool DestroyWindow(nint hwnd);

    [LibraryImport("user32.dll", EntryPoint = "DefWindowProcW")]
    public static partial nint DefWindowProc(nint hwnd, uint message, nuint wParam, nint lParam);

    [LibraryImport("user32.dll", EntryPoint = "GetMessageW")]
    public static partial int GetMessage(out Msg message, nint hwnd, uint filterMin, uint filterMax);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool TranslateMessage(in Msg message);

    [LibraryImport("user32.dll", EntryPoint = "DispatchMessageW")]
    public static partial nint DispatchMessage(in Msg message);

    [LibraryImport("user32.dll", EntryPoint = "SetWinEventHook")]
    public static partial nint SetWinEventHook(uint eventMin, uint eventMax, nint module, delegate* unmanaged<nint, uint, nint, int, int, uint, uint, void> callback, uint processId, uint threadId, uint flags);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool UnhookWinEvent(nint hook);

    public const uint SpiSetWorkArea = 0x002F;

    [LibraryImport("user32.dll", EntryPoint = "SystemParametersInfoW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SystemParametersInfo(uint action, uint parameter, ref Rect value, uint winIni);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsHungAppWindow(nint hwnd);

    public const int WhKeyboardLowLevel = 13;
    public const uint KeyboardHookInjected = 0x10;

    [LibraryImport("user32.dll", EntryPoint = "SetWindowsHookExW", SetLastError = true)]
    public static partial nint SetWindowsHookEx(int hookId, delegate* unmanaged<int, nuint, nint, nint> callback, nint module, uint threadId);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool UnhookWindowsHookEx(nint hook);

    [LibraryImport("user32.dll")]
    public static partial nint CallNextHookEx(nint hook, int code, nuint wParam, nint lParam);

    public static List<nint> TopLevelWindows()
    {
        var windows = new List<nint>(512);
        var handle = GCHandle.Alloc(windows);
        try
        {
            EnumWindows(&CollectHandle, GCHandle.ToIntPtr(handle));
        }
        finally
        {
            handle.Free();
        }

        return windows;
    }

    public static List<nint> ChildWindows(nint parent)
    {
        var children = new List<nint>(16);
        var handle = GCHandle.Alloc(children);
        try
        {
            EnumChildWindows(parent, &CollectHandle, GCHandle.ToIntPtr(handle));
        }
        finally
        {
            handle.Free();
        }

        return children;
    }

    public static List<nint> DisplayMonitors()
    {
        var monitors = new List<nint>(4);
        var handle = GCHandle.Alloc(monitors);
        try
        {
            EnumDisplayMonitors(0, 0, &CollectMonitor, GCHandle.ToIntPtr(handle));
        }
        finally
        {
            handle.Free();
        }

        return monitors;
    }

    [UnmanagedCallersOnly]
    private static int CollectHandle(nint hwnd, nint data)
    {
        ((List<nint>)GCHandle.FromIntPtr(data).Target!).Add(hwnd);
        return 1;
    }

    [UnmanagedCallersOnly]
    private static int CollectMonitor(nint monitor, nint hdc, Rect* rect, nint data)
    {
        ((List<nint>)GCHandle.FromIntPtr(data).Target!).Add(monitor);
        return 1;
    }

    public static string WindowText(nint hwnd)
    {
        var length = GetWindowTextLength(hwnd);
        if (length <= 0)
        {
            return "";
        }

        var buffer = new char[length + 1];
        fixed (char* text = buffer)
        {
            var copied = GetWindowText(hwnd, text, buffer.Length);
            return new string(text, 0, Math.Max(0, copied));
        }
    }

    public static string ClassName(nint hwnd)
    {
        var buffer = stackalloc char[256];
        var copied = GetClassName(hwnd, buffer, 256);
        return new string(buffer, 0, Math.Max(0, copied));
    }

    public static uint ProcessId(nint hwnd)
    {
        GetWindowThreadProcessId(hwnd, out var processId);
        return processId;
    }

    public static string MonitorDevice(nint monitor)
    {
        var info = MonitorInfoEx.Create();
        return GetMonitorInfo(monitor, ref info) ? info.DeviceName : "";
    }
}
