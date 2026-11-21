//! Emit the integrated core-owned serial PG state for external Nuked comparison.
//!
//! Arguments: <events.csv> <OPLL_Clock call count>
//! Event CSV columns: call_index, port (0=address, 1=data), value.

use sega_ym2413::{
    AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination, EnvelopeStage,
    Ym2413ChannelInitialState, Ym2413Core, Ym2413Operator, Ym2413PgSerialService,
    Ym2413SynthesisBank, Ym2413SynthesisInitialState, YM2413_INPUT_CLOCKS_PER_SLOT,
    slot_descriptor,
};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IoError, ErrorKind};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct Event {
    call_index: u64,
    port: u8,
    value: u8,
}

fn invalid_data(message: impl Into<String>) -> IoError {
    IoError::new(ErrorKind::InvalidData, message.into())
}

fn load_events(path: &Path, calls: u64) -> Result<Vec<Event>, Box<dyn Error>> {
    let input = BufReader::new(File::open(path)?);
    let mut events = Vec::new();

    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = trimmed.split(',').map(str::trim).collect();
        if fields.len() != 3 {
            return Err(invalid_data(format!(
                "line {}: expected call_index, port, value",
                line_index + 1
            ))
            .into());
        }
        let event = Event {
            call_index: fields[0].parse()?,
            port: fields[1].parse()?,
            value: fields[2].parse()?,
        };
        if event.port > 1 {
            return Err(invalid_data(format!(
                "line {}: unsupported YM2413 port {}",
                line_index + 1,
                event.port
            ))
            .into());
        }
        if event.call_index >= calls {
            return Err(invalid_data(format!(
                "line {}: event call {} is outside requested trace length {}",
                line_index + 1,
                event.call_index,
                calls
            ))
            .into());
        }
        if events
            .last()
            .is_some_and(|previous: &Event| previous.call_index > event.call_index)
        {
            return Err(invalid_data("events must be sorted by call_index").into());
        }
        events.push(event);
    }

    Ok(events)
}

fn reset_service() -> Result<Ym2413PgSerialService, Box<dyn Error>> {
    let damp_config = DampTransitionConfig::new(
        124,
        DampTransitionCoordination::IndependentPerOperator,
        AttackRate15Transition::SkipAttackToDecay,
    )
    .map_err(|error| invalid_data(format!("invalid damp configuration: {error:?}")))?;
    let initial = Ym2413SynthesisInitialState {
        channels: std::array::from_fn(|_| Ym2413ChannelInitialState {
            phase: [0; 2],
            envelope_level: [127; 2],
            envelope_stage: [EnvelopeStage::Damp; 2],
            envelope_key_on: [false; 2],
            feedback_history: [0; 2],
            previous_modulator_phase_index: None,
        }),
        pm_lfo_counter: 0,
        am_lfo_phase: 0,
        damp_config,
    };
    let bank = Ym2413SynthesisBank::with_initial_state(initial)
        .map_err(|error| invalid_data(format!("invalid synthesis bank state: {error:?}")))?;
    Ok(Ym2413PgSerialService::new(bank))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "usage: pg_serial_trace <events.csv> <OPLL_Clock call count>",
        )
        .into());
    }
    let calls: u64 = args[1].parse()?;
    let events = load_events(Path::new(&args[0]), calls)?;

    let mut core = Ym2413Core::new();
    let mut service = reset_service()?;
    let mut event_index = 0usize;

    for call_index in 0..calls {
        while event_index < events.len() && events[event_index].call_index == call_index {
            let event = events[event_index];
            match event.port {
                0 => core.write_address(event.value),
                1 => core.write_data(event.value),
                _ => unreachable!("event port was validated"),
            }
            event_index += 1;
        }

        if event_index < events.len() && events[event_index].call_index < call_index {
            return Err(invalid_data("an event call_index was skipped").into());
        }

        let cycle_before = core.slot_index();
        let lfo_vib_counter_before = service.lfo_vib_counter();
        core.advance_input_clocks_with_pg_service(
            YM2413_INPUT_CLOCKS_PER_SLOT,
            &mut service,
        )?;

        let mut phases = Vec::with_capacity(18);
        for slot in 0..18 {
            let descriptor = slot_descriptor(slot)
                .ok_or_else(|| invalid_data(format!("invalid physical PG slot {slot}")))?;
            let carrier = matches!(descriptor.operator, Ym2413Operator::Carrier);
            phases.push(
                service
                    .synthesis()
                    .phase_accumulator(descriptor.channel, carrier)
                    .map_err(|error| invalid_data(format!("invalid PG accumulator: {error:?}")))?,
            );
        }
        let phase_json = phases
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");

        println!(
            "{{\"call_index\":{},\"cycle_before\":{},\"cycle_after\":{},\"lfo_vib_counter_before\":{},\"lfo_vib_counter_after\":{},\"pg_increment\":{},\"pg_phase_next\":{},\"pg_output\":{},\"phases\":[{}]}}",
            call_index,
            cycle_before,
            core.slot_index(),
            lfo_vib_counter_before,
            service.lfo_vib_counter(),
            service.pg_increment(),
            service.pg_phase_next(),
            service.pg_output(),
            phase_json
        );
    }

    if event_index != events.len() {
        return Err(invalid_data("not all input events were consumed").into());
    }
    Ok(())
}
