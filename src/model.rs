use std::str::FromStr;
use serde::{Serialize, Deserialize};

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum Model {
    Gpt41106Preview,
    Gpt4VisionPreview,
    Gpt4,
    Gpt40613,
    Gpt432k,
    Gpt432k0613,
    #[default]
    Gpt35Turbo,
    Gpt35Turbo16k,
    Gpt35Turbo1106,
}

impl FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gpt-4-1106-preview" => Ok(Self::Gpt41106Preview),
            "gpt-4-vision-preview" => Ok(Self::Gpt4VisionPreview),
            "gpt-4" => Ok(Self::Gpt4),
            "gpt-4-0613" => Ok(Self::Gpt40613),
            "gpt-4-32k" => Ok(Self::Gpt432k),
            "gpt-4-32k-0613" => Ok(Self::Gpt432k0613),
            "gpt-3.5-turbo" => Ok(Self::Gpt35Turbo),
            "gpt-3.5-turbo-16k" => Ok(Self::Gpt35Turbo16k),
            "gpt-3.5-turbo-1106" => Ok(Self::Gpt35Turbo1106),
            _ => Err(format!("{} is not a valid model", s)),
        }
    }
}


impl ToString for Model {
    fn to_string(&self) -> String {
        match self {
            Self::Gpt41106Preview { .. } => String::from("gpt-4-1106-preview"),
            Self::Gpt4VisionPreview { .. } => String::from("gpt-4-vision-preview"),
            Self::Gpt4 { .. } => String::from("gpt-4"),
            Self::Gpt40613 { .. } => String::from("gpt-4-0613"),
            Self::Gpt432k { .. } => String::from("gpt-4-32k"),
            Self::Gpt432k0613 { .. } => String::from("gpt-4-32k-0613"),
            Self::Gpt35Turbo { .. } => String::from("gpt-3.5-turbo"),
            Self::Gpt35Turbo16k { .. } => String::from("gpt-3.5-turbo-16k"),
            Self::Gpt35Turbo1106 { .. } => String::from("gpt-3.5-turbo-1106"),
        }
    }
}

impl Serialize for Model {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Model {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Model {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Gpt41106Preview,
            Self::Gpt4VisionPreview,
            Self::Gpt4,
            Self::Gpt40613,
            Self::Gpt432k,
            Self::Gpt432k0613,
            Self::Gpt35Turbo,
            Self::Gpt35Turbo16k,
            Self::Gpt35Turbo1106,
        ]
    }

    pub fn cost(&self, prompt_tokens: usize, completion_tokens: usize) -> f64 {
        let (prompt_cost, completion_cost) = match self {
            Self::Gpt41106Preview => (0.01, 0.03),
            Self::Gpt4VisionPreview => (0.01, 0.03),
            Self::Gpt4 => (0.01, 0.03),
            Self::Gpt40613 => (0.01, 0.03),
            Self::Gpt432k => (0.06, 0.12),
            Self::Gpt432k0613 => (0.06, 0.12),
            Self::Gpt35Turbo => (0.0015, 0.002),
            Self::Gpt35Turbo16k => (0.0015, 0.002),
            Self::Gpt35Turbo1106 => (0.001, 0.002),
        };
        (prompt_tokens as f64).mul_add(
            prompt_cost / 1000.0,
            (completion_tokens as f64) * (completion_cost / 1000.0),
        )
    }

    pub const fn context_size(&self) -> usize {
        match self {
            Self::Gpt41106Preview => 128000,
            Self::Gpt4VisionPreview => 128000,
            Self::Gpt4 => 8192,
            Self::Gpt40613 => 8192,
            Self::Gpt432k => 32768,
            Self::Gpt432k0613 => 32768,
            Self::Gpt35Turbo => 4096,
            Self::Gpt35Turbo16k => 16385,
            Self::Gpt35Turbo1106 => 16385,
        }
    }
}