use crate::app::teams::ports::IDiceRoller;
use rand::Rng;

/// Le hasard réel. Le seul endroit du BC `teams` qui touche `rand`.
pub struct DiceAdapter;

impl IDiceRoller for DiceAdapter {
    fn d6(&self) -> u8 {
        rand::rng().random_range(1..=6)
    }

    fn d3(&self) -> u8 {
        rand::rng().random_range(1..=3)
    }

    fn two_d6(&self) -> (u8, u8) {
        let mut rng = rand::rng();
        (rng.random_range(1..=6), rng.random_range(1..=6))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On ne teste pas le hasard, seulement ses bornes : un dé hors bornes
    /// indexerait la table des erreurs coûteuses là où elle n'a rien.
    #[test]
    fn les_des_restent_dans_leurs_bornes() {
        let de = DiceAdapter;
        for _ in 0..200 {
            assert!((1..=6).contains(&de.d6()));
            assert!((1..=3).contains(&de.d3()));
            let (a, b) = de.two_d6();
            assert!((1..=6).contains(&a) && (1..=6).contains(&b));
        }
    }
}
