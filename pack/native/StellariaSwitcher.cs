using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Drawing.Text;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Windows.Forms;

[assembly: System.Reflection.AssemblyTitle("Stellaria Switcher")]
[assembly: System.Reflection.AssemblyProduct("Stellaria")]

static class Native
{
    public delegate IntPtr LowLevelKeyboardProc(int code, IntPtr wParam, IntPtr lParam);
    public delegate bool WindowEnumProc(IntPtr hWnd, IntPtr data);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X, Y; }

    [StructLayout(LayoutKind.Sequential)]
    public struct SIZE { public int Width, Height; }

    [StructLayout(LayoutKind.Sequential)]
    public struct KBDLLHOOKSTRUCT { public uint vkCode, scanCode, flags, time; public IntPtr dwExtraInfo; }

    [StructLayout(LayoutKind.Sequential)]
    public struct MONITORINFO { public int cbSize; public RECT rcMonitor, rcWork; public uint dwFlags; }

    [StructLayout(LayoutKind.Sequential, Pack = 1)]
    public struct BLENDFUNCTION { public byte BlendOp, BlendFlags, SourceConstantAlpha, AlphaFormat; }

    [StructLayout(LayoutKind.Sequential)]
    public struct DWM_THUMBNAIL_PROPERTIES
    {
        public int dwFlags;
        public RECT rcDestination;
        public RECT rcSource;
        public byte opacity;
        [MarshalAs(UnmanagedType.Bool)] public bool fVisible;
        [MarshalAs(UnmanagedType.Bool)] public bool fSourceClientAreaOnly;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct KEYBDINPUT { public ushort wVk, wScan; public uint dwFlags, time; public IntPtr dwExtraInfo; }

    [StructLayout(LayoutKind.Explicit)]
    public struct INPUT
    {
        [FieldOffset(0)] public uint type;
        [FieldOffset(8)] public KEYBDINPUT ki;
        [FieldOffset(8)] public MouseFiller filler;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct MouseFiller { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo; }

    [DllImport("user32.dll", SetLastError = true)] public static extern IntPtr SetWindowsHookEx(int id, LowLevelKeyboardProc callback, IntPtr module, uint threadId);
    [DllImport("user32.dll")] public static extern bool UnhookWindowsHookEx(IntPtr hook);
    [DllImport("user32.dll")] public static extern IntPtr CallNextHookEx(IntPtr hook, int code, IntPtr wParam, IntPtr lParam);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr GetModuleHandle(string name);
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int key);
    [DllImport("user32.dll")] public static extern uint SendInput(uint count, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("shcore.dll")] public static extern int GetDpiForMonitor(IntPtr monitor, int type, out uint dpiX, out uint dpiY);
    [DllImport("shell32.dll")] public static extern int SHQueryUserNotificationState(out int state);

    [DllImport("user32.dll")] public static extern bool EnumWindows(WindowEnumProc callback, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, WindowEnumProc callback, IntPtr data);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr hWnd, uint command);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr hWnd, int index);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr hWnd, StringBuilder text, int max);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int command);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint attach, uint attachTo, bool on);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("kernel32.dll")] public static extern uint GetCurrentProcessId();
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr hWnd, uint flags);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromPoint(POINT point, uint flags);
    [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFO info);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll", EntryPoint = "GetClassLongPtrW")] public static extern IntPtr GetClassLongPtr(IntPtr hWnd, int index);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern uint PrivateExtractIcons(string file, int index, int cx, int cy, IntPtr[] icons, uint[] ids, uint count, uint flags);
    [DllImport("user32.dll")] public static extern bool DestroyIcon(IntPtr icon);
    [DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(uint access, bool inherit, uint processId);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] public static extern bool QueryFullProcessImageName(IntPtr process, uint flags, StringBuilder name, ref uint size);
    [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);

    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hWnd, int attribute, out int value, int size);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hWnd, int attribute, out RECT value, int size);
    [DllImport("dwmapi.dll")] public static extern int DwmRegisterThumbnail(IntPtr destination, IntPtr source, out IntPtr thumbnail);
    [DllImport("dwmapi.dll")] public static extern int DwmUnregisterThumbnail(IntPtr thumbnail);
    [DllImport("dwmapi.dll")] public static extern int DwmUpdateThumbnailProperties(IntPtr thumbnail, ref DWM_THUMBNAIL_PROPERTIES properties);
    [DllImport("dwmapi.dll")] public static extern int DwmQueryThumbnailSourceSize(IntPtr thumbnail, out SIZE size);

    [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hWnd, IntPtr dc);
    [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleDC(IntPtr dc);
    [DllImport("gdi32.dll")] public static extern bool DeleteDC(IntPtr dc);
    [DllImport("gdi32.dll")] public static extern IntPtr SelectObject(IntPtr dc, IntPtr value);
    [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr value);
    [DllImport("user32.dll")] public static extern bool UpdateLayeredWindow(IntPtr hWnd, IntPtr dst, ref POINT position, ref SIZE size, IntPtr src, ref POINT source, int key, ref BLENDFUNCTION blend, int flags);

    public static bool Pressed(int key)
    {
        return (GetAsyncKeyState(key) & 0x8000) != 0;
    }

    public static RECT WorkArea(IntPtr monitor)
    {
        var info = new MONITORINFO { cbSize = Marshal.SizeOf(typeof(MONITORINFO)) };
        GetMonitorInfo(monitor, ref info);
        return info.rcWork;
    }

    public static float Scale(IntPtr monitor)
    {
        uint x, y;
        return GetDpiForMonitor(monitor, 0, out x, out y) == 0 && x > 0 ? x / 96f : 1f;
    }
}

sealed class Theme
{
    static readonly string FilePath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Stellaria", "switcher-theme.json");
    static readonly Regex Entry = new Regex("\"(\\w+)\"\\s*:\\s*(\"#([0-9a-fA-F]{6})\"|true|false)", RegexOptions.Compiled);

    DateTime loadedAt = DateTime.MinValue;

    public Color Surface = Hex("141218");
    public Color SurfaceContainer = Hex("211f26");
    public Color SurfaceContainerHigh = Hex("2b2930");
    public Color SurfaceContainerHighest = Hex("36343b");
    public Color SecondaryContainer = Hex("4a4458");
    public Color OnSecondaryContainer = Hex("e8def8");
    public Color PrimaryContainer = Hex("4f378b");
    public Color OnPrimaryContainer = Hex("eaddff");
    public Color OnSurface = Hex("e6e0e9");
    public Color OnSurfaceVariant = Hex("cac4d0");
    public Color Shadow = Hex("000000");
    public bool Transparent;

    static Color Hex(string value)
    {
        return Color.FromArgb(255, Color.FromArgb(int.Parse(value, NumberStyles.HexNumber)));
    }

    public void Refresh()
    {
        try
        {
            if (!File.Exists(FilePath)) return;
            DateTime stamp = File.GetLastWriteTimeUtc(FilePath);
            if (stamp == loadedAt) return;
            string text = File.ReadAllText(FilePath, Encoding.UTF8);
            foreach (Match match in Entry.Matches(text))
            {
                string name = match.Groups[1].Value;
                string raw = match.Groups[2].Value;
                if (name == "transparent") { Transparent = raw == "true"; continue; }
                if (!match.Groups[3].Success) continue;
                Color color = Hex(match.Groups[3].Value);
                switch (name)
                {
                    case "surface": Surface = color; break;
                    case "surfaceContainer": SurfaceContainer = color; break;
                    case "surfaceContainerHigh": SurfaceContainerHigh = color; break;
                    case "surfaceContainerHighest": SurfaceContainerHighest = color; break;
                    case "secondaryContainer": SecondaryContainer = color; break;
                    case "onSecondaryContainer": OnSecondaryContainer = color; break;
                    case "primaryContainer": PrimaryContainer = color; break;
                    case "onPrimaryContainer": OnPrimaryContainer = color; break;
                    case "onSurface": OnSurface = color; break;
                    case "onSurfaceVariant": OnSurfaceVariant = color; break;
                    case "shadow": Shadow = color; break;
                }
            }
            loadedAt = stamp;
        }
        catch (IOException) { }
        catch (UnauthorizedAccessException) { }
    }
}

sealed class SwitcherItem
{
    public IntPtr Handle;
    public string Title;
    public string Exe;
    public bool Minimized;
    public Bitmap Icon;
    public Bitmap LargeIcon;
}

static class WindowList
{
    static readonly HashSet<string> ShellClasses = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
    {
        "Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd", "Windows.UI.Core.CoreWindow"
    };

    static readonly HashSet<string> IgnoredProcesses = new HashSet<string>(StringComparer.OrdinalIgnoreCase) { "zebar" };

    static bool IsTask(IntPtr hWnd, string className)
    {
        if (!Native.IsWindowVisible(hWnd) || Native.GetWindowTextLength(hWnd) == 0) return false;
        int exStyle = Native.GetWindowLong(hWnd, -20);
        bool appWindow = (exStyle & 0x40000) != 0;
        if (!appWindow && ((exStyle & 0x80) != 0 || (exStyle & 0x08000000) != 0 || Native.GetWindow(hWnd, 4) != IntPtr.Zero)) return false;
        int cloaked;
        if (Native.DwmGetWindowAttribute(hWnd, 14, out cloaked, 4) == 0 && cloaked != 0) return false;
        return !ShellClasses.Contains(className);
    }

    static uint HostedProcess(IntPtr frame, uint framePid)
    {
        uint hosted = framePid;
        Native.EnumChildWindows(frame, (child, data) =>
        {
            uint pid;
            Native.GetWindowThreadProcessId(child, out pid);
            if (pid == framePid) return true;
            hosted = pid;
            return false;
        }, IntPtr.Zero);
        return hosted;
    }

    static string ExecutablePath(uint pid)
    {
        IntPtr process = Native.OpenProcess(0x1000, false, pid);
        if (process == IntPtr.Zero) return "";
        try
        {
            var name = new StringBuilder(1024);
            uint size = (uint)name.Capacity;
            return Native.QueryFullProcessImageName(process, 0, name, ref size) ? name.ToString() : "";
        }
        finally { Native.CloseHandle(process); }
    }

    public static List<SwitcherItem> Enumerate(IconCache icons, float scale)
    {
        var items = new List<SwitcherItem>();
        var exes = new Dictionary<uint, string>();
        uint self = Native.GetCurrentProcessId();
        IntPtr foreground = Native.GetForegroundWindow();

        Native.EnumWindows((hWnd, data) =>
        {
            var className = new StringBuilder(256);
            Native.GetClassName(hWnd, className, className.Capacity);
            if (!IsTask(hWnd, className.ToString())) return true;

            uint pid;
            Native.GetWindowThreadProcessId(hWnd, out pid);
            if (pid == self) return true;
            if (string.Equals(className.ToString(), "ApplicationFrameWindow", StringComparison.Ordinal)) pid = HostedProcess(hWnd, pid);

            string exe;
            if (!exes.TryGetValue(pid, out exe)) exes[pid] = exe = ExecutablePath(pid);
            if (exe.Length > 0 && IgnoredProcesses.Contains(Path.GetFileNameWithoutExtension(exe))) return true;

            var title = new StringBuilder(512);
            Native.GetWindowText(hWnd, title, title.Capacity);
            var item = new SwitcherItem { Handle = hWnd, Title = title.ToString(), Exe = exe, Minimized = Native.IsIconic(hWnd) };
            icons.Resolve(item, scale);
            if (hWnd == foreground) items.Insert(0, item);
            else items.Add(item);
            return true;
        }, IntPtr.Zero);

        icons.Trim(items);
        return items;
    }

    public static void Activate(IntPtr hWnd)
    {
        if (!Native.IsWindow(hWnd)) return;
        if (Native.IsIconic(hWnd)) Native.ShowWindow(hWnd, 9);
        if (Native.SetForegroundWindow(hWnd) && Native.GetForegroundWindow() == hWnd) return;

        IntPtr foreground = Native.GetForegroundWindow();
        uint pid;
        uint target = Native.GetWindowThreadProcessId(foreground, out pid);
        uint current = Native.GetCurrentThreadId();
        bool attached = target != 0 && target != current && Native.AttachThreadInput(current, target, true);
        try
        {
            Native.BringWindowToTop(hWnd);
            Native.SetForegroundWindow(hWnd);
        }
        finally
        {
            if (attached) Native.AttachThreadInput(current, target, false);
        }
    }

    public static void Close(IntPtr hWnd)
    {
        Native.PostMessage(hWnd, 0x0010, IntPtr.Zero, IntPtr.Zero);
    }
}

sealed class IconCache
{
    sealed class Entry
    {
        public IntPtr Source;
        public int Size;
        public Bitmap Icon;
        public Bitmap LargeIcon;
    }

    readonly Dictionary<IntPtr, Entry> windows = new Dictionary<IntPtr, Entry>();
    readonly Dictionary<string, Bitmap> executables = new Dictionary<string, Bitmap>(StringComparer.OrdinalIgnoreCase);

    static Bitmap FromHandle(IntPtr icon)
    {
        if (icon == IntPtr.Zero) return null;
        try
        {
            using (var managed = Icon.FromHandle(icon)) return managed.ToBitmap();
        }
        catch (ArgumentException) { return null; }
        catch (ExternalException) { return null; }
    }

    static IntPtr WindowIcon(IntPtr hWnd)
    {
        IntPtr result;
        foreach (int kind in new[] { 1, 2, 0 })
        {
            if (Native.SendMessageTimeout(hWnd, 0x007F, new IntPtr(kind), IntPtr.Zero, 0x0002, 50, out result) != IntPtr.Zero && result != IntPtr.Zero) return result;
        }
        IntPtr classIcon = Native.GetClassLongPtr(hWnd, -14);
        return classIcon != IntPtr.Zero ? classIcon : Native.GetClassLongPtr(hWnd, -34);
    }

    Bitmap ExecutableIcon(string exe, int size)
    {
        if (string.IsNullOrEmpty(exe)) return null;
        string key = exe + "|" + size.ToString(CultureInfo.InvariantCulture);
        Bitmap cached;
        if (executables.TryGetValue(key, out cached)) return cached;
        Bitmap bitmap = null;
        var handles = new IntPtr[1];
        var ids = new uint[1];
        if (Native.PrivateExtractIcons(exe, 0, size, size, handles, ids, 1, 0) == 1 && handles[0] != IntPtr.Zero)
        {
            bitmap = FromHandle(handles[0]);
            Native.DestroyIcon(handles[0]);
        }
        executables[key] = bitmap;
        return bitmap;
    }

    public void Resolve(SwitcherItem item, float scale)
    {
        int small = (int)Math.Round(24 * scale);
        int large = (int)Math.Round(64 * scale);
        IntPtr source = WindowIcon(item.Handle);
        Entry entry;
        if (!windows.TryGetValue(item.Handle, out entry) || entry.Source != source || entry.Size != small)
        {
            Bitmap exeSmall = ExecutableIcon(item.Exe, small);
            entry = new Entry
            {
                Source = source,
                Size = small,
                Icon = FromHandle(source) ?? exeSmall,
                LargeIcon = ExecutableIcon(item.Exe, large)
            };
            if (entry.LargeIcon == null) entry.LargeIcon = entry.Icon;
            windows[item.Handle] = entry;
        }
        item.Icon = entry.Icon;
        item.LargeIcon = entry.LargeIcon;
    }

    public void Trim(List<SwitcherItem> alive)
    {
        var keep = new HashSet<IntPtr>();
        foreach (var item in alive) keep.Add(item.Handle);
        var stale = new List<IntPtr>();
        foreach (var handle in windows.Keys) if (!keep.Contains(handle)) stale.Add(handle);
        foreach (var handle in stale) windows.Remove(handle);
    }
}

sealed class Tile
{
    public SwitcherItem Item;
    public Rectangle Bounds;
    public Rectangle Preview;
    public IntPtr Thumbnail;
}

sealed class SwitcherWindow : NativeWindow, IDisposable
{
    const int ShadowLayers = 14;

    readonly Switcher owner;
    readonly Theme theme;
    readonly PrivateFontCollection fonts = new PrivateFontCollection();
    readonly FontFamily brandFamily;
    readonly FontFamily fallbackFamily;
    readonly System.Windows.Forms.Timer fade = new System.Windows.Forms.Timer { Interval = 15 };

    List<Tile> tiles = new List<Tile>();
    int selected;
    int hovered = -1;
    float scale = 1f;
    Point origin;
    Size size;
    IntPtr memoryDc;
    IntPtr bitmap;
    IntPtr previousBitmap;
    byte opacity;
    DateTime fadeStart;
    Native.POINT shownCursor;
    bool cursorMoved;

    public bool Visible { get; private set; }
    public int Columns { get; private set; }

    public SwitcherWindow(Switcher owner, Theme theme)
    {
        this.owner = owner;
        this.theme = theme;
        string font = Path.Combine(Path.GetDirectoryName(Application.ExecutablePath), "..", "fonts", "gsflex-switcher.ttf");
        if (File.Exists(font))
        {
            fonts.AddFontFile(font);
            brandFamily = fonts.Families.Length > 0 ? fonts.Families[0] : null;
        }
        fallbackFamily = new FontFamily("Segoe UI Variable Text");
        if (brandFamily == null) brandFamily = fallbackFamily;
        fade.Tick += (sender, args) => StepFade();

        CreateHandle(new CreateParams
        {
            Caption = "Stellaria Switcher",
            Style = unchecked((int)0x80000000),
            ExStyle = 0x00080000 | 0x00000080 | 0x08000000 | 0x00000008
        });
    }

    static bool BrandCovers(string text)
    {
        foreach (char c in text)
        {
            if (c <= 0x024F || (c >= 0x1E00 && c <= 0x1EFF) || (c >= 0x2000 && c <= 0x206F) || c == 0x20AC || c == 0x2122) continue;
            return false;
        }
        return true;
    }

    static GraphicsPath Rounded(RectangleF rect, float radius)
    {
        var path = new GraphicsPath();
        float d = Math.Min(radius * 2, Math.Min(rect.Width, rect.Height));
        if (d <= 0) { path.AddRectangle(rect); return path; }
        path.AddArc(rect.X, rect.Y, d, d, 180, 90);
        path.AddArc(rect.Right - d, rect.Y, d, d, 270, 90);
        path.AddArc(rect.Right - d, rect.Bottom - d, d, d, 0, 90);
        path.AddArc(rect.X, rect.Bottom - d, d, d, 90, 90);
        path.CloseFigure();
        return path;
    }

    static Color Alpha(Color color, double alpha)
    {
        return Color.FromArgb((int)Math.Round(Math.Max(0, Math.Min(1, alpha)) * 255), color);
    }

    int Px(float value)
    {
        return (int)Math.Round(value * scale);
    }

    public void Present(List<SwitcherItem> items, int index, IntPtr monitor)
    {
        ReleaseThumbnails();
        scale = Native.Scale(monitor);
        Native.RECT work = Native.WorkArea(monitor);
        int workWidth = work.Right - work.Left;
        int workHeight = work.Bottom - work.Top;

        int margin = Px(32);
        int cardPadding = Px(15);
        int gap = Px(8);
        int tilePadding = Px(10);
        int header = Px(28);
        int headerGap = Px(8);
        float aspect = 10f / 16f;
        int previewWidth = Px(232);
        int count = items.Count;
        int columns, rows, tileWidth, tileHeight, previewHeight;

        while (true)
        {
            previewHeight = (int)Math.Round(previewWidth * aspect);
            tileWidth = previewWidth + tilePadding * 2;
            tileHeight = tilePadding + header + headerGap + previewHeight + tilePadding;
            int available = (int)(workWidth * 0.92) - margin * 2 - cardPadding * 2;
            columns = Math.Max(1, Math.Min(count, (available + gap) / (tileWidth + gap)));
            rows = (count + columns - 1) / columns;
            int height = rows * tileHeight + (rows - 1) * gap + cardPadding * 2 + margin * 2;
            if (height <= workHeight * 0.9 || previewWidth <= Px(112)) break;
            previewWidth -= Px(16);
        }

        Columns = columns;
        int cardWidth = columns * tileWidth + (columns - 1) * gap + cardPadding * 2;
        int cardHeight = rows * tileHeight + (rows - 1) * gap + cardPadding * 2;
        size = new Size(cardWidth + margin * 2, cardHeight + margin * 2);
        origin = new Point(work.Left + (workWidth - size.Width) / 2, work.Top + (workHeight - size.Height) / 2);

        tiles = new List<Tile>(count);
        for (int i = 0; i < count; i++)
        {
            int row = i / columns;
            int inRow = row == rows - 1 ? count - row * columns : columns;
            int rowOffset = (columns - inRow) * (tileWidth + gap) / 2;
            int x = margin + cardPadding + rowOffset + (i % columns) * (tileWidth + gap);
            int y = margin + cardPadding + row * (tileHeight + gap);
            var tile = new Tile
            {
                Item = items[i],
                Bounds = new Rectangle(x, y, tileWidth, tileHeight),
                Preview = new Rectangle(x + tilePadding, y + tilePadding + header + headerGap, previewWidth, previewHeight)
            };
            tiles.Add(tile);
        }

        selected = index;
        hovered = -1;
        Native.GetCursorPos(out shownCursor);
        cursorMoved = false;
        Render();

        Native.SetWindowPos(Handle, new IntPtr(-1), origin.X, origin.Y, size.Width, size.Height, 0x0010 | 0x0040);
        Native.ShowWindow(Handle, 4);
        Visible = true;

        foreach (var tile in tiles) RegisterThumbnail(tile);
        opacity = 0;
        fadeStart = DateTime.UtcNow;
        Commit(0);
        fade.Start();
    }

    void RegisterThumbnail(Tile tile)
    {
        if (tile.Item.Minimized) return;
        IntPtr thumbnail;
        if (Native.DwmRegisterThumbnail(Handle, tile.Item.Handle, out thumbnail) != 0) return;
        tile.Thumbnail = thumbnail;
        UpdateThumbnail(tile, 0);
    }

    void UpdateThumbnail(Tile tile, byte alpha)
    {
        if (tile.Thumbnail == IntPtr.Zero) return;
        Native.SIZE source;
        if (Native.DwmQueryThumbnailSourceSize(tile.Thumbnail, out source) != 0 || source.Width <= 0 || source.Height <= 0) return;

        var crop = new Native.RECT { Left = 0, Top = 0, Right = source.Width, Bottom = source.Height };
        Native.RECT window, frame;
        if (Native.GetWindowRect(tile.Item.Handle, out window) && Native.DwmGetWindowAttribute(tile.Item.Handle, 9, out frame, Marshal.SizeOf(typeof(Native.RECT))) == 0)
        {
            double windowWidth = window.Right - window.Left;
            double windowHeight = window.Bottom - window.Top;
            if (windowWidth > 0 && windowHeight > 0)
            {
                double sx = source.Width / windowWidth;
                double sy = source.Height / windowHeight;
                crop.Left = (int)Math.Round(Math.Max(0, frame.Left - window.Left) * sx);
                crop.Top = (int)Math.Round(Math.Max(0, frame.Top - window.Top) * sy);
                crop.Right = source.Width - (int)Math.Round(Math.Max(0, window.Right - frame.Right) * sx);
                crop.Bottom = source.Height - (int)Math.Round(Math.Max(0, window.Bottom - frame.Bottom) * sy);
                if (crop.Right <= crop.Left || crop.Bottom <= crop.Top) crop = new Native.RECT { Left = 0, Top = 0, Right = source.Width, Bottom = source.Height };
            }
        }

        double cropWidth = crop.Right - crop.Left;
        double cropHeight = crop.Bottom - crop.Top;
        double fit = Math.Min(tile.Preview.Width / cropWidth, tile.Preview.Height / cropHeight);
        int width = Math.Max(1, (int)Math.Round(cropWidth * fit));
        int height = Math.Max(1, (int)Math.Round(cropHeight * fit));
        int left = tile.Preview.X + (tile.Preview.Width - width) / 2;
        int top = tile.Preview.Y + (tile.Preview.Height - height) / 2;

        var properties = new Native.DWM_THUMBNAIL_PROPERTIES
        {
            dwFlags = 0x1 | 0x2 | 0x4 | 0x8 | 0x10,
            rcDestination = new Native.RECT { Left = left, Top = top, Right = left + width, Bottom = top + height },
            rcSource = crop,
            opacity = alpha,
            fVisible = true,
            fSourceClientAreaOnly = false
        };
        Native.DwmUpdateThumbnailProperties(tile.Thumbnail, ref properties);
    }

    void ReleaseThumbnails()
    {
        foreach (var tile in tiles)
        {
            if (tile.Thumbnail == IntPtr.Zero) continue;
            Native.DwmUnregisterThumbnail(tile.Thumbnail);
            tile.Thumbnail = IntPtr.Zero;
        }
    }

    public void Select(int index)
    {
        if (!Visible || index == selected) { selected = index; return; }
        selected = index;
        Render();
        // Selection changes only alter the layered surface. Existing DWM thumbnails
        // keep the same bounds and opacity, so updating every thumbnail here causes
        // needless compositor work on every Tab press.
        Commit(opacity, false);
    }

    public void Hide()
    {
        fade.Stop();
        ReleaseThumbnails();
        if (Visible) Native.ShowWindow(Handle, 0);
        Visible = false;
        tiles = new List<Tile>();
    }

    void StepFade()
    {
        double t = Math.Min(1, (DateTime.UtcNow - fadeStart).TotalMilliseconds / 80.0);
        double eased = 1 - Math.Pow(1 - t, 3);
        byte alpha = (byte)Math.Round(eased * 255);
        Commit(alpha);
        if (t >= 1) fade.Stop();
    }

    void Commit(byte alpha, bool updateThumbnails = true)
    {
        opacity = alpha;
        if (memoryDc == IntPtr.Zero) return;
        IntPtr screen = Native.GetDC(IntPtr.Zero);
        try
        {
            var position = new Native.POINT { X = origin.X, Y = origin.Y };
            var extent = new Native.SIZE { Width = size.Width, Height = size.Height };
            var source = new Native.POINT();
            var blend = new Native.BLENDFUNCTION { BlendOp = 0, BlendFlags = 0, SourceConstantAlpha = alpha, AlphaFormat = 1 };
            Native.UpdateLayeredWindow(Handle, screen, ref position, ref extent, memoryDc, ref source, 0, ref blend, 2);
        }
        finally { Native.ReleaseDC(IntPtr.Zero, screen); }
        if (updateThumbnails)
            foreach (var tile in tiles) UpdateThumbnail(tile, alpha);
    }

    void ReleaseSurface()
    {
        if (memoryDc == IntPtr.Zero) return;
        Native.SelectObject(memoryDc, previousBitmap);
        Native.DeleteObject(bitmap);
        Native.DeleteDC(memoryDc);
        memoryDc = IntPtr.Zero;
        bitmap = IntPtr.Zero;
    }

    void Render()
    {
        int margin = Px(32);
        float radius = 25 * scale;
        float tileRadius = 17 * scale;
        float previewRadius = 12 * scale;
        Color surface = theme.Transparent ? Alpha(theme.Surface, 0.85) : theme.Surface;

        using (var canvas = new Bitmap(size.Width, size.Height, PixelFormat.Format32bppArgb))
        {
            using (var g = Graphics.FromImage(canvas))
            {
                g.SmoothingMode = SmoothingMode.AntiAlias;
                g.PixelOffsetMode = PixelOffsetMode.HighQuality;
                g.InterpolationMode = InterpolationMode.HighQualityBicubic;
                g.TextRenderingHint = TextRenderingHint.AntiAliasGridFit;
                g.Clear(Color.Transparent);

                var card = new RectangleF(margin, margin, size.Width - margin * 2, size.Height - margin * 2);
                for (int i = ShadowLayers; i >= 1; i--)
                {
                    float spread = i * (margin - Px(6)) / (float)ShadowLayers;
                    var layer = new RectangleF(card.X - spread, card.Y - spread + Px(6), card.Width + spread * 2, card.Height + spread * 2);
                    double strength = 0.035 * (1 - (i - 1) / (double)ShadowLayers);
                    using (var path = Rounded(layer, radius + spread))
                    using (var brush = new SolidBrush(Alpha(theme.Shadow, strength)))
                        g.FillPath(brush, path);
                }

                using (var path = Rounded(card, radius))
                using (var brush = new SolidBrush(surface))
                    g.FillPath(brush, path);

                using (var format = new StringFormat(StringFormatFlags.NoWrap) { Trimming = StringTrimming.EllipsisCharacter, LineAlignment = StringAlignment.Center })
                using (var brandFont = new Font(brandFamily, 13.5f * scale, FontStyle.Regular, GraphicsUnit.Pixel))
                using (var fallbackFont = new Font(fallbackFamily, 13f * scale, FontStyle.Regular, GraphicsUnit.Pixel))
                using (var badgeFont = new Font(brandFamily, 26f * scale, FontStyle.Regular, GraphicsUnit.Pixel))
                using (var smallBadgeFont = new Font(brandFamily, 11f * scale, FontStyle.Regular, GraphicsUnit.Pixel))
                {
                    for (int i = 0; i < tiles.Count; i++)
                    {
                        var tile = tiles[i];
                        bool isSelected = i == selected;
                        if (isSelected || i == hovered)
                        {
                            Color fill = isSelected ? theme.SecondaryContainer : Alpha(theme.OnSurface, 0.08);
                            using (var path = Rounded(tile.Bounds, tileRadius))
                            using (var brush = new SolidBrush(fill))
                                g.FillPath(brush, path);
                        }

                        using (var path = Rounded(tile.Preview, previewRadius))
                        using (var brush = new SolidBrush(isSelected ? Alpha(theme.Surface, 0.55) : theme.SurfaceContainer))
                            g.FillPath(brush, path);

                        int pad = tile.Preview.X - tile.Bounds.X;
                        int iconSize = Px(20);
                        int headerTop = tile.Bounds.Y + pad;
                        var iconRect = new Rectangle(tile.Bounds.X + pad, headerTop + (Px(28) - iconSize) / 2, iconSize, iconSize);
                        if (tile.Item.Icon != null) g.DrawImage(tile.Item.Icon, iconRect);
                        else DrawBadge(g, iconRect, tile.Item, Px(6), smallBadgeFont);

                        var textRect = new RectangleF(iconRect.Right + Px(8), headerTop, tile.Bounds.Right - pad - iconRect.Right - Px(8), Px(28));
                        Color textColor = isSelected ? theme.OnSecondaryContainer : theme.OnSurface;
                        using (var brush = new SolidBrush(textColor))
                            g.DrawString(tile.Item.Title, BrandCovers(tile.Item.Title) ? brandFont : fallbackFont, brush, textRect, format);

                        if (tile.Item.Minimized)
                        {
                            int large = Px(56);
                            var largeRect = new Rectangle(tile.Preview.X + (tile.Preview.Width - large) / 2, tile.Preview.Y + (tile.Preview.Height - large) / 2, large, large);
                            if (tile.Item.LargeIcon != null) g.DrawImage(tile.Item.LargeIcon, largeRect);
                            else DrawBadge(g, largeRect, tile.Item, Px(17), badgeFont);
                        }
                    }
                }
            }

            ReleaseSurface();
            IntPtr screen = Native.GetDC(IntPtr.Zero);
            try
            {
                memoryDc = Native.CreateCompatibleDC(screen);
                bitmap = canvas.GetHbitmap(Color.FromArgb(0));
                previousBitmap = Native.SelectObject(memoryDc, bitmap);
            }
            finally { Native.ReleaseDC(IntPtr.Zero, screen); }
        }
    }

    void DrawBadge(Graphics g, Rectangle rect, SwitcherItem item, int radius, Font font)
    {
        using (var path = Rounded(rect, radius))
        using (var brush = new SolidBrush(theme.PrimaryContainer))
            g.FillPath(brush, path);
        string name = Path.GetFileNameWithoutExtension(item.Exe ?? "");
        if (name.Length == 0) name = item.Title;
        string letter = name.Length > 0 ? char.ToUpperInvariant(name[0]).ToString() : "?";
        using (var brush = new SolidBrush(theme.OnPrimaryContainer))
        using (var format = new StringFormat { Alignment = StringAlignment.Center, LineAlignment = StringAlignment.Center })
            g.DrawString(letter, font, brush, rect, format);
    }

    int HitTest(IntPtr lParam)
    {
        int x = (short)(lParam.ToInt64() & 0xFFFF);
        int y = (short)((lParam.ToInt64() >> 16) & 0xFFFF);
        for (int i = 0; i < tiles.Count; i++) if (tiles[i].Bounds.Contains(x, y)) return i;
        return -1;
    }

    protected override void WndProc(ref Message m)
    {
        switch (m.Msg)
        {
            case 0x0021:
                m.Result = new IntPtr(3);
                return;
            case 0x0200:
            {
                Native.POINT cursor;
                Native.GetCursorPos(out cursor);
                if (!cursorMoved && Math.Abs(cursor.X - shownCursor.X) + Math.Abs(cursor.Y - shownCursor.Y) < 3) break;
                cursorMoved = true;
                int index = HitTest(m.LParam);
                if (index >= 0 && index != selected) owner.Hover(index);
                break;
            }
            case 0x0202:
            {
                int index = HitTest(m.LParam);
                if (index >= 0) owner.Choose(index);
                return;
            }
            case 0x0205:
            {
                int index = HitTest(m.LParam);
                if (index >= 0) owner.CloseAt(index);
                return;
            }
            case 0x8001:
                owner.ExitThread();
                return;
            case 0x8002:
                owner.InitializeBegin();
                return;
        }
        base.WndProc(ref m);
    }

    public void Dispose()
    {
        Hide();
        ReleaseSurface();
        fade.Dispose();
        fonts.Dispose();
        DestroyHandle();
    }
}

sealed class Switcher : ApplicationContext
{
    readonly Native.LowLevelKeyboardProc hookProc;
    readonly Theme theme = new Theme();
    readonly IconCache icons = new IconCache();
    readonly SwitcherWindow window;
    readonly System.Windows.Forms.Timer reveal = new System.Windows.Forms.Timer { Interval = 20 };
    readonly System.Windows.Forms.Timer watchdog = new System.Windows.Forms.Timer { Interval = 40 };
    readonly System.Windows.Forms.Timer rehook = new System.Windows.Forms.Timer { Interval = 30000 };
    readonly System.Windows.Forms.Timer warmup = new System.Windows.Forms.Timer { Interval = 750 };
    readonly RegisteredWaitHandle stopWait;

    static readonly IntPtr MaskSignature = new IntPtr(0x43415354);

    IntPtr hook;
    bool active;
    List<SwitcherItem> items = new List<SwitcherItem>();
    int selected;
    IntPtr monitor;
    bool loading;
    int pendingDirection;

    public Switcher(EventWaitHandle stop)
    {
        hookProc = HookCallback;
        theme.Refresh();
        window = new SwitcherWindow(this, theme);
        reveal.Tick += (sender, args) => { reveal.Stop(); if (active) Reveal(); };
        watchdog.Tick += (sender, args) => { if (active && !Native.Pressed(0x12)) Commit(); };
        rehook.Tick += (sender, args) => { if (!active) InstallHook(); };
        warmup.Tick += (sender, args) =>
        {
            warmup.Stop();
            if (!active) WindowList.Enumerate(icons, Native.Scale(TargetMonitor()));
        };
        InstallHook();
        rehook.Start();
        warmup.Start();
        IntPtr target = window.Handle;
        stopWait = ThreadPool.RegisterWaitForSingleObject(stop, (state, timedOut) => Native.PostMessage(target, 0x8001, IntPtr.Zero, IntPtr.Zero), null, Timeout.Infinite, true);
    }

    void InstallHook()
    {
        IntPtr fresh = Native.SetWindowsHookEx(13, hookProc, Native.GetModuleHandle(null), 0);
        if (fresh == IntPtr.Zero) return;
        if (hook != IntPtr.Zero) Native.UnhookWindowsHookEx(hook);
        hook = fresh;
    }

    IntPtr HookCallback(int code, IntPtr wParam, IntPtr lParam)
    {
        if (code >= 0)
        {
            var key = (Native.KBDLLHOOKSTRUCT)Marshal.PtrToStructure(lParam, typeof(Native.KBDLLHOOKSTRUCT));
            if (key.dwExtraInfo != MaskSignature)
            {
                bool handled = false;
                try { handled = Handle(key.vkCode, wParam.ToInt32(), key.flags); }
                catch (Exception) { Cancel(); }
                if (handled) return new IntPtr(1);
            }
        }
        return Native.CallNextHookEx(hook, code, wParam, lParam);
    }

    static bool PassThrough()
    {
        if (Native.Pressed(0x11) || Native.Pressed(0x5B) || Native.Pressed(0x5C)) return true;
        int state;
        return Native.SHQueryUserNotificationState(out state) == 0 && state == 3;
    }

    bool Handle(uint vk, int message, uint flags)
    {
        bool down = message == 0x0100 || message == 0x0104;
        bool alt = (flags & 0x20) != 0;

        if (vk == 0x09)
        {
            if (!active)
            {
                if (!down || !alt || PassThrough()) return false;
                return Begin(Native.Pressed(0x10) ? -1 : 1);
            }
            if (down) Step(Native.Pressed(0x10) ? -1 : 1);
            return true;
        }

        if (!active) return false;

        if (vk == 0xA4 || vk == 0xA5 || vk == 0x12)
        {
            if (!down) Commit();
            return false;
        }

        switch (vk)
        {
            case 0x1B: if (down) Cancel(); return true;
            case 0x0D: if (down) Commit(); return true;
            case 0x25: if (down) Step(-1); return true;
            case 0x27: if (down) Step(1); return true;
            case 0x26: if (down) Step(-Columns()); return true;
            case 0x28: if (down) Step(Columns()); return true;
            case 0x2E: if (down) CloseAt(selected); return true;
        }
        return false;
    }

    int Columns()
    {
        return Math.Max(1, window.Columns);
    }

    static void Mask()
    {
        var inputs = new Native.INPUT[2];
        inputs[0].type = 1;
        inputs[0].ki.wVk = 0xE8;
        inputs[0].ki.dwExtraInfo = MaskSignature;
        inputs[1].type = 1;
        inputs[1].ki.wVk = 0xE8;
        inputs[1].ki.dwExtraInfo = MaskSignature;
        inputs[1].ki.dwFlags = 0x0002;
        Native.SendInput(2, inputs, Marshal.SizeOf(typeof(Native.INPUT)));
    }

    static IntPtr TargetMonitor()
    {
        IntPtr foreground = Native.GetForegroundWindow();
        if (foreground != IntPtr.Zero && Native.IsWindowVisible(foreground) && !Native.IsIconic(foreground)) return Native.MonitorFromWindow(foreground, 2);
        Native.POINT cursor;
        Native.GetCursorPos(out cursor);
        return Native.MonitorFromPoint(cursor, 2);
    }

    bool Begin(int direction)
    {
        monitor = TargetMonitor();
        active = true;
        loading = true;
        pendingDirection = direction;
        Mask();
        watchdog.Start();
        Native.PostMessage(window.Handle, 0x8002, IntPtr.Zero, IntPtr.Zero);
        return true;
    }

    public void InitializeBegin()
    {
        if (!active || !loading) return;

        theme.Refresh();
        items = WindowList.Enumerate(icons, Native.Scale(monitor));
        if (!active) return;
        if (items.Count == 0)
        {
            Finish();
            return;
        }

        int direction = pendingDirection;
        int first = items[0].Handle == Native.GetForegroundWindow() && items.Count > 1 ? 1 : 0;
        selected = direction >= 0 ? first : items.Count - 1;
        int extraSteps = direction >= 0 ? direction - 1 : direction + 1;
        selected = ((selected + extraSteps) % items.Count + items.Count) % items.Count;
        loading = false;
        reveal.Start();
    }

    void Reveal()
    {
        window.Present(items, selected, monitor);
    }

    void Step(int delta)
    {
        if (loading)
        {
            pendingDirection += delta;
            return;
        }
        if (items.Count == 0) return;
        selected = ((selected + delta) % items.Count + items.Count) % items.Count;
        if (window.Visible) window.Select(selected);
    }

    public void Hover(int index)
    {
        if (!active || index < 0 || index >= items.Count) return;
        selected = index;
        window.Select(index);
    }

    public void Choose(int index)
    {
        if (!active || index < 0 || index >= items.Count) return;
        selected = index;
        Commit();
    }

    public void CloseAt(int index)
    {
        if (!active || index < 0 || index >= items.Count) return;
        WindowList.Close(items[index].Handle);
        items.RemoveAt(index);
        if (items.Count == 0) { Cancel(); return; }
        if (selected >= items.Count) selected = items.Count - 1;
        if (window.Visible) window.Present(items, selected, monitor);
    }

    void Finish()
    {
        active = false;
        loading = false;
        pendingDirection = 0;
        reveal.Stop();
        watchdog.Stop();
        window.Hide();
    }

    void Commit()
    {
        if (!active) return;
        IntPtr target = selected >= 0 && selected < items.Count ? items[selected].Handle : IntPtr.Zero;
        Finish();
        if (target != IntPtr.Zero) WindowList.Activate(target);
    }

    void Cancel()
    {
        if (!active) return;
        Finish();
    }

    protected override void ExitThreadCore()
    {
        stopWait.Unregister(null);
        if (hook != IntPtr.Zero) Native.UnhookWindowsHookEx(hook);
        hook = IntPtr.Zero;
        reveal.Dispose();
        watchdog.Dispose();
        rehook.Dispose();
        warmup.Dispose();
        window.Dispose();
        base.ExitThreadCore();
    }
}

static class Program
{
    [STAThread]
    static int Main(string[] args)
    {
        bool quit = args.Length > 0 && string.Equals(args[0], "--quit", StringComparison.OrdinalIgnoreCase);
        using (var stop = new EventWaitHandle(false, EventResetMode.ManualReset, @"Local\StellariaSwitcherStop"))
        {
            if (quit)
            {
                stop.Set();
                return 0;
            }
            bool created;
            using (var mutex = new Mutex(true, @"Local\StellariaSwitcher", out created))
            {
                if (!created) return 1;
                stop.Reset();
                Native.SetProcessDpiAwarenessContext(new IntPtr(-4));
                Application.Run(new Switcher(stop));
                mutex.ReleaseMutex();
            }
        }
        return 0;
    }
}
