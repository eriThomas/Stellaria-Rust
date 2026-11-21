using Stellaria.Helper.Desktop;

namespace Stellaria.Helper.Tests.Modules;

public sealed class WindowPreviewCacheTests
{
    [Fact]
    public void Cached_preview_is_available_after_a_window_is_minimized()
    {
        var cache = new WindowPreviewCache(4);

        cache.Store(42, "last-visible-frame");

        Assert.True(cache.TryGet(42, out var image));
        Assert.Equal("last-visible-frame", image);
    }

    [Fact]
    public void Oldest_preview_is_evicted_at_capacity()
    {
        var cache = new WindowPreviewCache(2);
        cache.Store(1, "first");
        cache.Store(2, "second");

        cache.Store(3, "third");

        Assert.False(cache.TryGet(1, out _));
        Assert.True(cache.TryGet(2, out _));
        Assert.True(cache.TryGet(3, out _));
        Assert.Equal(2, cache.Count);
    }
}
