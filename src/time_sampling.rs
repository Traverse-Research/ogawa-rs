use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::BufReader;
use std::rc::Rc;

use crate::chunks::DataChunk;
use crate::result::*;

const ACYCLIC_NUM_SAMPLES: u32 = u32::MAX;
const ACYCLIC_TIME_PER_CYCLE: f64 = f64::MAX / 32.0;
#[derive(Debug)]
pub struct TimeSamplingType {
    num_samples_per_cycle: u32,
    time_per_cycle: f64,
}
#[derive(Debug)]
pub struct TimeSampling {
    sampling_type: TimeSamplingType,
    samples: Vec<f64>,
}

pub(crate) fn read_time_samplings_and_max(
    data: &DataChunk,
    reader: &mut BufReader<File>,
) -> Result<(Vec<Rc<TimeSampling>>, Vec<i64>)> {
    let mut buffer = vec![0u8; data.size as usize];
    data.read(0, reader, &mut buffer)?;
    let mut buffer = std::io::Cursor::new(buffer);

    let mut out_max_samples = vec![];
    let mut out_time_samples = vec![];

    loop {
        if buffer.position() == data.size {
            break;
        }

        let max_sample = buffer.read_u32::<LittleEndian>()?;
        out_max_samples.push(max_sample as i64);
        let time_per_cycle = buffer.read_f64::<LittleEndian>()?;
        let num_samples_per_cycle = buffer.read_u32::<LittleEndian>()?;

        let mut samples = vec![0.0f64; num_samples_per_cycle as usize];
        buffer
            .read_f64_into::<LittleEndian>(&mut samples)
            .map_err(|_| ParsingError::InvalidAlembicFile)?;

        let sampling_type = if time_per_cycle == f64::MAX / 32.0 {
            TimeSamplingType {
                num_samples_per_cycle: ACYCLIC_NUM_SAMPLES,
                time_per_cycle: ACYCLIC_TIME_PER_CYCLE,
            }
        } else {
            TimeSamplingType {
                num_samples_per_cycle,
                time_per_cycle,
            }
        };

        out_time_samples.push(Rc::new(TimeSampling {
            sampling_type,
            samples,
        }));
    }

    Ok((out_time_samples, out_max_samples))
}
