//! The provider states of the engine's snapshots, which the applet's About
//! page shows: off for a provider that is switched off, not available
//! after a start or read that found nothing to read, and available
//! otherwise, also while a started provider has not reported yet.
//!
//! The engine runs on the virtual clock of `puron_testkit::eng` with fake
//! providers, so that every time is exact. Each start of a provider
//! begins a new generation, and the engine drops a result that belongs to
//! an earlier one.

use std::sync::Arc;

use puron_core::rows::Provider::{self, *};
use puron_core::settings::Settings;
use puron_engine::engine::{Engine, ProviderState, Snapshot};
use puron_engine::registry::ReadOutcome;
use puron_testkit::eng::{adapters, r1, r2, Fake, ReadStep, StartStep, VirtualClock};

use ProviderState::{Available, NotAvailable, Off};

/// The time limit of the test, in seconds.
const LIMIT: u64 = 5;

/// Settings with the providers in `ps` switched on, every other provider
/// switched off, and a poll interval of 5 seconds.
fn on(ps: &[Provider]) -> Settings {
    Settings {
        provider_lm_sensors: ps.contains(&LmSensors),
        provider_liquidctl: ps.contains(&Liquidctl),
        provider_nvidia: ps.contains(&Nvidia),
        provider_udisks2: ps.contains(&Udisks2),
        provider_nvme: ps.contains(&NvmeCli),
        poll_interval_secs: 5,
        ..Settings::default()
    }
}

/// An engine on a virtual clock and the latest snapshot taken from it.
struct Run {
    clock: Arc<VirtualClock>,
    e: Engine,
    last: Option<Snapshot>,
}

impl Run {
    /// Sends new provider switches and waits until the engine has handled
    /// them, without moving the clock.
    fn set(&self, s: &Settings) {
        self.e.set_providers(s);
        self.clock.settle();
    }

    /// The provider states of the latest snapshot that the engine has
    /// published, in the engine's provider order: lm-sensors, liquidctl,
    /// NVIDIA, udisks2, nvme-cli.
    fn states(&mut self) -> Vec<ProviderState> {
        if let Some(s) = self.e.try_next() {
            self.last = Some(s);
        }
        let s = self.last.as_ref().expect("a snapshot");
        let order: Vec<Provider> = s.providers.iter().map(|p| p.0).collect();
        assert_eq!(order, [LmSensors, Liquidctl, Nvidia, Udisks2, NvmeCli]);
        s.providers.iter().map(|p| p.1).collect()
    }
}

/// lm-sensors, liquidctl and NVIDIA start; udisks2 and nvme-cli are off.
/// lm-sensors reads at once, liquidctl's start finds nothing, and NVIDIA's
/// first read finds nothing and its second gives readings. Then
/// lm-sensors is switched off; on again with a read that hangs; off and
/// on once more, after which the hung read of the earlier generation ends
/// as not available. Last, NVIDIA is switched off and on and its start
/// panics. Expected: each state as the comments below give it, and a
/// report of an earlier generation changes nothing.
#[test]
fn c_tests_36() {
    puron_testkit::limit("C-tests-36", LIMIT, || {
        let clock = VirtualClock::new();
        let fake = |p| Fake::new(clock.dyn_clock(), p);
        let lm = fake(LmSensors).reads([ReadStep::Gives(r1(LmSensors))]);
        let lq = fake(Liquidctl).starts([StartStep::Unavailable]);
        let nv = fake(Nvidia).reads([
            ReadStep::Gives(ReadOutcome::Unavailable),
            ReadStep::Gives(r2(Nvidia)),
        ]);
        let fakes = [lm.clone(), lq, nv.clone(), fake(Udisks2), fake(NvmeCli)];
        let mut run = Run {
            clock: clock.clone(),
            e: Engine::new(clock.dyn_clock()),
            last: None,
        };
        let three = on(&[LmSensors, Liquidctl, Nvidia]);
        let no_lm = on(&[Liquidctl, Nvidia]);
        run.e.start(&three, adapters(&fakes));
        clock.settle();
        // The first reports, with no clock movement.
        assert_eq!(
            run.states(),
            [Available, NotAvailable, NotAvailable, Off, Off],
            "after the first reports"
        );
        // NVIDIA's second read gives readings.
        clock.advance_to(5.0);
        assert_eq!(run.states()[2], Available, "after NVIDIA's second read");
        // lm-sensors switched off.
        run.set(&no_lm);
        assert_eq!(run.states()[0], Off, "lm-sensors off");
        // On again: its read hangs, and NVIDIA's read publishes a
        // snapshot. A started provider that has not reported is available.
        lm.script_reads([ReadStep::Hangs]);
        run.set(&three);
        assert_eq!(
            run.states()[0],
            Available,
            "lm-sensors on, its read hanging"
        );
        // Off and on again; then the hung read of the earlier generation
        // ends as not available, which the engine drops. The new read
        // hangs too, and NVIDIA's next read publishes a snapshot.
        run.set(&no_lm);
        run.set(&three);
        lm.release_read(ReadOutcome::Unavailable);
        clock.settle();
        clock.advance_to(10.0);
        assert_eq!(
            run.states()[0],
            Available,
            "after a report of an earlier generation"
        );
        // NVIDIA off and on, and its start panics.
        nv.script_starts([StartStep::Panics]);
        run.set(&on(&[LmSensors, Liquidctl]));
        run.set(&three);
        assert_eq!(
            run.states(),
            [Available, NotAvailable, NotAvailable, Off, Off],
            "after NVIDIA's start panics"
        );
        // Let the hanging read end, and no later read hang, so that the
        // engine can shut down.
        lm.script_reads([ReadStep::Gives(r1(LmSensors))]);
        lm.release_read(r1(LmSensors));
        clock.settle();
        run.e.shutdown();
    });
}
