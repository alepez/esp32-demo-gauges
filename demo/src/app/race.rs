use std::time::Duration;

use crate::app::gates::Gates;
use crate::svc::CoordinatedInstant;

#[derive(Debug, Default, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Race {
    pub start_time: Option<CoordinatedInstant>,
    pub finish_time: Option<CoordinatedInstant>,
    pub duration: Option<Duration>,
}

impl Race {
    pub fn set_gates(&mut self, gates: &Gates) {
        let start_time = gates.start_gate().last_activation_time;
        let finish_time = gates.finish_gate().last_activation_time;

        // Always override start time if it is defined
        if let Some(start_time) = start_time {
            self.start_time = Some(start_time);
        }

        // Override finish time only if it is not already defined
        if self.finish_time.is_none() {
            if let Some(finish_time) = finish_time {
                self.finish_time = Some(finish_time);
            }
        }

        if let Some(start_time) = start_time {
            if let Some(finish_time) = self.finish_time {
                if start_time > finish_time {
                    // When start gate is activated after finish gate, it means
                    // we have a new race, so we must reset.
                    self.finish_time = None;
                    self.duration = None;
                } else {
                    // When finish gate is activated after start gate, it means
                    // the race has just ended and we can calculate the duration.
                    let duration_millis = finish_time.as_millis() - start_time.as_millis();
                    self.duration = Some(Duration::from_millis(duration_millis as u64));
                }
            }
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use crate::app::gates::Gate;

    use super::*;

    fn make_active_gate(time_ms: i32) -> Gate {
        let t = Some(CoordinatedInstant::from_millis(time_ms));
        Gate {
            active: true,
            last_activation_time: t,
            last_beacon_time: t,
        }
    }

    fn make_inactive_gate(time_ms: i32) -> Gate {
        let t = Some(CoordinatedInstant::from_millis(time_ms));
        Gate {
            active: false,
            last_activation_time: t,
            last_beacon_time: t,
        }
    }

    fn make_never_activated_gate() -> Gate {
        Gate {
            active: false,
            last_activation_time: None,
            last_beacon_time: None,
        }
    }

    #[test]
    fn test_race_default() {
        let race = Race::default();
        assert_debug_snapshot!(race);
    }

    #[test]
    fn test_race_with_start_gate_active() {
        let mut race = Race::default();
        race.set_gates(&Gates::new([
            make_active_gate(10_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_never_activated_gate(),
        ]));
        assert_debug_snapshot!(race);
    }

    #[test]
    fn test_race_with_start_and_finish_gates_active() {
        let mut race = Race::default();
        race.set_gates(&Gates::new([
            make_active_gate(10_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_never_activated_gate(),
        ]));
        race.set_gates(&Gates::new([
            make_active_gate(10_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_active_gate(20_000),
        ]));
        assert_debug_snapshot!(race);
    }

    #[test]
    fn test_race_with_start_after_finish() {
        let mut race = Race::default();
        race.set_gates(&Gates::new([
            make_active_gate(10_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_never_activated_gate(),
        ]));
        race.set_gates(&Gates::new([
            make_active_gate(10_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_active_gate(20_000),
        ]));
        race.set_gates(&Gates::new([
            make_active_gate(30_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_active_gate(20_000),
        ]));
        assert_debug_snapshot!(race);
    }

    #[test]
    fn test_race_with_two_finish_activations() {
        let mut race = Race::default();
        race.set_gates(&Gates::new([
            make_active_gate(10_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_never_activated_gate(),
        ]));
        race.set_gates(&Gates::new([
            make_inactive_gate(10_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_active_gate(20_000),
        ]));
        race.set_gates(&Gates::new([
            make_inactive_gate(10_000),
            make_never_activated_gate(),
            make_never_activated_gate(),
            make_active_gate(30_000),
        ]));

        // Even if the finish gate has been activated again, the duration is
        // calculated start gate activation and first finish gate activation.
        assert_eq!(race.duration, Some(Duration::from_secs(10)));

        assert_debug_snapshot!(race);
    }
}
