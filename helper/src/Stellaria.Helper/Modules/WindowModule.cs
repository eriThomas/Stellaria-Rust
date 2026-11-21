using System.Diagnostics;
using System.Text.Json.Nodes;
using Stellaria.Helper.Desktop;
using Stellaria.Helper.Imaging;
using Stellaria.Helper.Interop;
using Stellaria.Helper.Protocol;

namespace Stellaria.Helper.Modules;

internal sealed class WindowModule : IHelperModule
{
    public const int PreviewWidth = 640;
    public const uint PrintFullContent = 2;

    public string Name => "window";

    public Task StartAsync(ModuleContext context)
    {
        context.Router.Register("window.act", async (request, _) =>
        {
            var action = request.RequireString("action");
            var done = await context.Loop.InvokeAsync(() => Act(context.Foreground.LastUserWindow, action)).ConfigureAwait(false);
            if (!done)
            {
                throw new HelperException(ErrorCode.Failed, "Windows refused the window action.");
            }

            return null;
        });
        context.Router.Register("window.preview", async (request, _) =>
        {
            var window = request.Params["id"] is JsonValue id && id.TryGetValue<string>(out var taskId)
                ? TasksModule.ParseId(taskId)
                : context.Foreground.LastUserWindow;
            var capture = await context.Loop.InvokeAsync(() => Capture(window)).ConfigureAwait(false);
            var image = capture is null ? null : Convert.ToBase64String(await ImageCodec.EncodeJpegAsync(capture, PreviewWidth).ConfigureAwait(false));
            return new JsonObject { ["image"] = image };
        });
        context.Router.Register("capture.record", async (_, _) =>
        {
            if (!await KeyboardInput.SendToWindowAsync(context.Loop, context.Foreground.LastUserWindow, KeyboardInput.RecordClip).ConfigureAwait(false))
            {
                throw new HelperException(ErrorCode.Failed, "There is no window to record.");
            }

            return null;
        });
        context.Router.Register("shell.taskView", async (_, _) =>
        {
            await context.Loop.InvokeAsync(() => KeyboardInput.Press(KeyboardInput.TaskView)).ConfigureAwait(false);
            return null;
        });
        context.Router.Register("capture.region", (_, _) =>
        {
            if (!ShellExecute.Open("ms-screenclip:"))
            {
                throw new HelperException(ErrorCode.Failed, "Windows could not open the snipping tool.");
            }

            return Task.FromResult<JsonNode?>(null);
        });
        return Task.CompletedTask;
    }

    public static bool Act(nint window, string action)
    {
        if (action is not ("close" or "minimize" or "maximize" or "focus"))
        {
            throw new HelperException(ErrorCode.InvalidParams, "Unknown window action.");
        }

        if (window == 0 || !User32.IsWindow(window))
        {
            throw new HelperException(ErrorCode.NotFound, "There is no window to act on.");
        }

        switch (action)
        {
            case "close":
                return User32.PostMessage(window, User32.WmClose, 0, 0);
            case "minimize":
                User32.ShowWindow(window, User32.SwMinimize);
                return true;
            case "maximize":
                User32.ShowWindow(window, User32.IsZoomed(window) ? User32.SwRestore : User32.SwMaximize);
                return true;
            default:
                if (User32.IsIconic(window))
                {
                    User32.ShowWindow(window, User32.SwRestore);
                }

                return User32.SetForegroundWindow(window);
        }
    }

    private static PixelImage? Capture(nint window)
    {
        if (window == 0 || !User32.IsWindow(window) || User32.IsIconic(window) || !User32.GetWindowRect(window, out var bounds) || bounds.IsEmpty)
        {
            return null;
        }

        // NanaZip's modern window crashes inside NanaZip.Modern.dll when another
        // process forces a PW_RENDERFULLCONTENT capture. Keep the task available,
        // but let the UI fall back to its application icon instead of PrintWindow.
        if (IsCaptureBlocked(window))
        {
            return null;
        }

        var frame = Dwmapi.FrameBounds(window) ?? bounds;
        return WindowCapture.PrintRegion(window, bounds.Width, bounds.Height, PrintFullContent,
            frame.Left - bounds.Left, frame.Top - bounds.Top, frame.Width, frame.Height, PreviewWidth);
    }

    private static bool IsCaptureBlocked(nint window)
    {
        try
        {
            var processId = User32.ProcessId(window);
            if (processId == 0)
            {
                return false;
            }

            using var process = Process.GetProcessById((int)processId);
            return process.ProcessName.Contains("NanaZip", StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            return false;
        }
    }
}
