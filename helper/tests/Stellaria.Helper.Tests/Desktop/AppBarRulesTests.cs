using Stellaria.Helper.Desktop;
using Stellaria.Helper.Interop;
using Stellaria.Helper.Tests.TestSupport;

namespace Stellaria.Helper.Tests.Desktop;

public sealed class AppBarRulesTests
{
    private static readonly Rect s_monitor = new(0, 0, 3440, 1440);

    [Fact]
    public void Thin_windows_are_classified_by_the_edge_they_hug()
    {
        Assert.Equal(DockEdge.Left, AppBarRules.Classify(1, new Rect(0, 10, 48, 1430), s_monitor)!.Edge);
        Assert.Equal(DockEdge.Top, AppBarRules.Classify(1, new Rect(0, 0, 3440, 10), s_monitor)!.Edge);
        Assert.Equal(DockEdge.Right, AppBarRules.Classify(1, new Rect(3430, 10, 3440, 1430), s_monitor)!.Edge);
        Assert.Equal(DockEdge.Bottom, AppBarRules.Classify(1, new Rect(0, 1430, 3440, 1440), s_monitor)!.Edge);
    }

    [Fact]
    public void A_window_thicker_than_a_dock_is_never_treated_as_one()
    {
        Assert.Null(AppBarRules.Classify(1, new Rect(48, 240, 3488, 840), s_monitor));
    }

    [Fact]
    public void A_dock_is_reserved_when_the_work_area_stops_at_its_inner_side()
    {
        var bar = AppBarRules.Classify(1, new Rect(0, 10, 48, 1430), s_monitor)!;

        Assert.True(AppBarRules.IsReserved(bar, new Rect(48, 10, 3430, 1430)));
        Assert.False(AppBarRules.IsReserved(bar, new Rect(0, 0, 3440, 1392)));
    }

    [Fact]
    public void A_reservation_by_another_app_on_another_edge_does_not_hide_a_lost_dock()
    {
        var bottom = AppBarRules.Classify(1, new Rect(0, 1430, 3440, 1440), s_monitor)!;
        var left = AppBarRules.Classify(2, new Rect(0, 10, 48, 1430), s_monitor)!;
        var workWithOnlyTaskbar = new Rect(0, 0, 3440, 1392);

        Assert.True(AppBarRules.IsReserved(bottom, workWithOnlyTaskbar));
        Assert.False(AppBarRules.IsReserved(left, workWithOnlyTaskbar));
    }

    [Fact]
    public void A_dock_pushed_inward_by_stacked_reservations_no_longer_touches_its_edge()
    {
        var side = new Rect(-1920, 0, 0, 1080);
        var healthy = AppBarRules.Classify(1, new Rect(-1920, 10, -1872, 1070), side)!;
        var pushed = AppBarRules.Classify(2, new Rect(-1152, 160, -1104, 1220), side)!;

        Assert.True(AppBarRules.TouchesItsEdge(healthy, side));
        Assert.False(AppBarRules.TouchesItsEdge(pushed, side));
    }

    [Fact]
    public void A_pushed_left_dock_on_a_negative_monitor_is_snapped_to_the_physical_edge()
    {
        var monitor = new Rect(-2560, 3, 0, 1443);
        var pushed = AppBarRules.Classify(1, new Rect(-2512, 4, -2464, 1444), monitor)!;

        Assert.Equal(new Rect(-2560, 3, -2512, 1443), AppBarRules.SnapToEdge(pushed, monitor));
    }

    [Theory]
    [InlineData(1, 0, 0, 3440, 48)]
    [InlineData(2, 3392, 0, 3440, 1440)]
    [InlineData(3, 0, 1392, 3440, 1440)]
    public void Snapping_also_covers_the_other_edges(int edge, int left, int top, int right, int bottom)
    {
        var dock = new Dock(1, new Rect(100, 100, 148, 148), (DockEdge)edge, 48);

        Assert.Equal(new Rect(left, top, right, bottom), AppBarRules.SnapToEdge(dock, s_monitor));
    }

    [Theory]
    [InlineData("Zebar - stellaria / bar", true)]
    [InlineData("Zebar - stellaria / edge", true)]
    [InlineData("Zebar - stellaria / dashboard", false)]
    [InlineData("stellaria / bar - Notepad", false)]
    public void Only_bar_and_edge_widgets_are_docks(string title, bool dock)
    {
        Assert.Equal(dock, AppBarRules.IsDockTitle(title));
    }

    [Fact]
    public void The_breaker_trips_on_the_fifth_repair_within_a_minute()
    {
        var time = new ManualTimeProvider(DateTimeOffset.UnixEpoch);
        var breaker = new RepairBreaker(5, TimeSpan.FromMinutes(1), TimeSpan.FromMinutes(5), time);

        for (var repair = 0; repair < 4; repair++)
        {
            Assert.False(breaker.RecordRepair());
            time.Advance(TimeSpan.FromSeconds(2));
        }

        Assert.True(breaker.RecordRepair());
        Assert.True(breaker.IsPaused);
        time.Advance(TimeSpan.FromMinutes(5));
        Assert.False(breaker.IsPaused);
    }

    [Fact]
    public void Occasional_repairs_never_trip_the_breaker()
    {
        var time = new ManualTimeProvider(DateTimeOffset.UnixEpoch);
        var breaker = new RepairBreaker(5, TimeSpan.FromMinutes(1), TimeSpan.FromMinutes(5), time);

        for (var repair = 0; repair < 20; repair++)
        {
            Assert.False(breaker.RecordRepair());
            time.Advance(TimeSpan.FromSeconds(20));
        }
    }
}
