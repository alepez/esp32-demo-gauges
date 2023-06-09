#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct LocalInstant(i32);

impl LocalInstant {
    pub fn from_millis(ms: i32) -> Self {
        Self(ms)
    }

    pub fn as_millis(&self) -> i32 {
        self.0
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct LocalClock {
    start: std::time::Instant,
}

impl Default for LocalClock {
    fn default() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}

impl LocalClock {
    pub fn now(&self) -> Option<LocalInstant> {
        let t = std::time::Instant::now().checked_duration_since(self.start)?;
        let t_ms = t.as_millis();

        // milliseconds, 32 bits, max 24 days (enough!)
        if t_ms < (i32::MAX as u128) {
            Some(LocalInstant(t_ms as i32))
        } else {
            None
        }
    }
}

#[derive(
    Default,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CoordinatedInstant(i32);

impl CoordinatedInstant {
    pub fn from_millis(ms: i32) -> Self {
        Self(ms)
    }

    pub fn as_millis(&self) -> i32 {
        self.0
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct CoordinatedClock {
    clock: LocalClock,
    offset: LocalOffset,
}

impl CoordinatedClock {
    pub fn new(clock: LocalClock, offset: LocalOffset) -> Self {
        Self { clock, offset }
    }

    pub fn now(&self) -> CoordinatedInstant {
        let t = self.clock.now().expect("Cannot get time");
        CoordinatedInstant::from_millis(t.as_millis() + self.offset.as_millis())
    }

    pub fn offset(&self) -> LocalOffset {
        self.offset
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct LocalOffset(i32);

impl LocalOffset {
    pub fn from_millis(ms: i32) -> Self {
        Self(ms)
    }

    pub fn as_millis(&self) -> i32 {
        self.0
    }
}

pub fn calculate_clock_offset(
    coordinator_time: CoordinatedInstant,
    local_time: LocalInstant,
) -> LocalOffset {
    let c = coordinator_time.as_millis();
    let l = local_time.as_millis();
    LocalOffset::from_millis(c - l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_clock_offset_when_coordinator_started_before_gate() {
        let coord_time = CoordinatedInstant::from_millis(60_000);
        let local_time = LocalInstant::from_millis(10_000);
        let offset = calculate_clock_offset(coord_time, local_time);
        assert_eq!(offset, LocalOffset::from_millis(50_000));
    }

    #[test]
    fn test_calculate_clock_offset_when_coordinator_started_after_gate() {
        let coord_time = CoordinatedInstant::from_millis(60_000);
        let local_time = LocalInstant::from_millis(110_000);
        let offset = calculate_clock_offset(coord_time, local_time);
        assert_eq!(offset, LocalOffset::from_millis(-50_000));
    }
}
