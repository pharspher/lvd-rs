use std::borrow::Borrow;
use std::fmt;

use esp_idf_hal::adc::attenuation::DB_6;
use esp_idf_hal::adc::Adc;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::gpio::ADCPin;

pub struct MoistureSensor<'a, A, P>
where
    A: Adc + 'a,
    P: ADCPin + 'a,
    &'a AdcDriver<'a, A>: Borrow<AdcDriver<'a, <P as ADCPin>::Adc>>,
{
    adc: &'a AdcDriver<'a, A>,
    channel: AdcChannelDriver<'a, P, &'a AdcDriver<'a, A>>,
    recent_samples: [u16; 3],
    sample_idx: usize,
}

impl<'a, A, P> MoistureSensor<'a, A, P>
where
    A: Adc + 'a,
    P: ADCPin + 'a,
    &'a AdcDriver<'a, A>: Borrow<AdcDriver<'a, <P as ADCPin>::Adc>>,
{
    pub fn new(adc: &'a AdcDriver<'a, A>, pin: P) -> Self {
        let config = AdcChannelConfig {
            attenuation: DB_6,
            ..Default::default()
        };
        let channel = AdcChannelDriver::new(adc, pin, &config)
            .expect("Failed to create ADC channel driver");
        Self {
            adc,
            channel,
            recent_samples: [0; 3],
            sample_idx: 0,
        }
    }

    pub fn read(&mut self) -> (u16, MoistureLevel) {
        let value = self.adc.read(&mut self.channel).expect("Failed to read ADC");
        (value, MoistureLevel::from_value(value))
    }

    pub fn read_avg(&mut self) -> Option<(u16, MoistureLevel)> {
        let (value, _) = self.read();
        self.recent_samples[self.sample_idx] = value;
        self.sample_idx = (self.sample_idx + 1) % self.recent_samples.len();

        if self.recent_samples.iter().any(|&v| v == 0) {
            return None;
        }

        let sum: u32 = self.recent_samples.iter().map(|&v| v as u32).sum();
        let avg = (sum / self.recent_samples.len() as u32) as u16;

        Some((avg, MoistureLevel::from_value(avg)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MoistureLevel {
    VeryWet,
    Wet,
    Normal,
    Dry,
    VeryDry,
}

impl fmt::Display for MoistureLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            MoistureLevel::VeryWet => "Super wet",
            MoistureLevel::Wet => "Quite wet",
            MoistureLevel::Normal => "Normal   ",
            MoistureLevel::Dry => "Quite dry",
            MoistureLevel::VeryDry => "Super dry",
        };
        write!(f, "{}", label)
    }
}

impl MoistureLevel {
    pub fn from_value(value: u16) -> Self {
        let moisture_min = 900;
        let moisture_max = 1412;
        let moisture_step = (moisture_max - moisture_min) / 5;

        match value {
            v if v <= moisture_min + moisture_step => MoistureLevel::VeryWet,
            v if v <= moisture_min + moisture_step * 2 => MoistureLevel::Wet,
            v if v <= moisture_min + moisture_step * 3 => MoistureLevel::Normal,
            v if v <= moisture_min + moisture_step * 4 => MoistureLevel::Dry,
            _ => MoistureLevel::VeryDry,
        }
    }
}
