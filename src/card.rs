use crate::token_pool::TokenPool;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardType {
    Land,
    Artifact,
    Enchantment,
    Creature,
    Other(String),
}

impl CardType {
    pub fn parse(value: &str) -> Self {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.contains("land") {
            Self::Land
        } else if normalized.contains("artifact") {
            Self::Artifact
        } else if normalized.contains("enchantment") {
            Self::Enchantment
        } else if normalized.contains("creature") {
            Self::Creature
        } else {
            Self::Other(value.trim().to_string())
        }
    }
}

impl std::fmt::Display for CardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Land => write!(f, "Land"),
            Self::Artifact => write!(f, "Artifact"),
            Self::Enchantment => write!(f, "Enchantment"),
            Self::Creature => write!(f, "Creature"),
            Self::Other(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub card_type: CardType,
    pub mana_cost: Option<String>,
    pub colors: Option<String>,
    pub oracle_text: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub is_commander: bool,
    pub is_partner: bool,
    pub token_pools: Vec<TokenPool>,
}
