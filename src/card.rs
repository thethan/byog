use crate::token_pool::TokenPool;

/// Represents the commander/partner role of a card.
///
/// - `None` – ordinary card, no commander or partner status.
/// - `Commander` – the card is the sole commander.
/// - `PartnerCommander` – the card is a commander that also has the Partner ability,
///   allowing it to share the Command zone with one other `PartnerCommander`.
/// - `PartnerOnly` – the card has the Partner ability but is not currently designated
///   as a commander (edge-case; retained for completeness).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoleStatus {
    #[default]
    None,
    Commander,
    PartnerCommander,
    PartnerOnly,
}

impl RoleStatus {
    /// Derives the role from the two boolean CSV fields `is_commander` and `is_partner`.
    pub fn from_bools(is_commander: bool, is_partner: bool) -> Self {
        match (is_commander, is_partner) {
            (true, true) => Self::PartnerCommander,
            (true, false) => Self::Commander,
            (false, true) => Self::PartnerOnly,
            (false, false) => Self::None,
        }
    }

    /// Returns `true` when the card occupies (or should occupy) the Command zone.
    pub fn is_commander(self) -> bool {
        matches!(self, Self::Commander | Self::PartnerCommander)
    }

    /// Returns `true` when the card has the Partner ability.
    pub fn is_partner(self) -> bool {
        matches!(self, Self::PartnerCommander | Self::PartnerOnly)
    }
}

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
    pub role: RoleStatus,
    pub token_pools: Vec<TokenPool>,
    pub starting_pile: Option<String>,
}

impl Card {
    /// Returns `true` when the card is a commander (sole or partner).
    pub fn is_commander(&self) -> bool {
        self.role.is_commander()
    }

    /// Returns `true` when the card has the Partner ability.
    pub fn is_partner(&self) -> bool {
        self.role.is_partner()
    }
}
