using System.Collections.Concurrent;

namespace Stellaria.Helper.Desktop;

internal sealed class WindowPreviewCache(int capacity)
{
    private readonly ConcurrentDictionary<nint, Entry> _items = new();
    private long _sequence;

    public int Count => _items.Count;

    public bool TryGet(nint window, out string? image)
    {
        if (_items.TryGetValue(window, out var entry))
        {
            image = entry.Image;
            return true;
        }

        image = null;
        return false;
    }

    public void Store(nint window, string image)
    {
        if (window == 0 || string.IsNullOrEmpty(image))
        {
            return;
        }

        _items[window] = new Entry(image, Interlocked.Increment(ref _sequence));
        var excess = _items.Count - Math.Max(1, capacity);
        if (excess <= 0)
        {
            return;
        }

        foreach (var stale in _items.OrderBy(item => item.Value.Sequence).Take(excess))
        {
            _items.TryRemove(stale.Key, out _);
        }
    }

    private sealed record Entry(string Image, long Sequence);
}
