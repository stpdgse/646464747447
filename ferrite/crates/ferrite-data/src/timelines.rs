//! Timelines: the repeating schedules of the world clocks and their named time markers.

pub use crate::generated::timelines::TIMELINES;

/// A named moment in a timeline (`minecraft:noon` is 6000 ticks into the day).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeMarker {
    pub name: &'static str,
    pub ticks: i64,
    /// Whether commands offer it.
    pub show_in_commands: bool,
}

/// A schedule that repeats every `period_ticks` of its clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeline {
    pub name: &'static str,
    /// The world clock the timeline follows.
    pub clock: &'static str,
    pub period_ticks: Option<i64>,
    pub markers: &'static [TimeMarker],
}

/// The timeline with this name.
pub fn timeline(name: &str) -> Option<&'static Timeline> {
    TIMELINES.iter().find(|t| t.name == name)
}

/// The time marker with this name, in any timeline: its timeline and the marker.
pub fn time_marker(name: &str) -> Option<(&'static Timeline, &'static TimeMarker)> {
    TIMELINES.iter().find_map(|t| t.markers.iter().find(|m| m.name == name).map(|m| (t, m)))
}
