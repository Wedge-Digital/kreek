//! Ce que la couche web a besoin de savoir des BCs, et qu'elle ne va pas
//! chercher elle-même.
//!
//! # Le sens de la dépendance, qui est neuf ici
//!
//! Le précédent du dépôt est `ISpacesHostLayout` : un BC déclare ce qu'il attend
//! de l'hôte, l'hôte l'implémente. **Celui-ci va dans l'autre sens** — c'est le
//! layout qui consomme une donnée de BC, et il passe par un port pour ne pas
//! importer `competitions`.
//!
//! `src/web/` atteignait jusqu'ici `auth`, `spaces`, `news` (ses routes
//! seulement) et `shared_kernel`. Ce port est la première dépendance vers
//! `competitions`, et elle reste tenue : seul
//! `src/infrastructure/web/hors_calendrier_adapter.rs` importe le BC source.

use async_trait::async_trait;

/// Faut-il cacher l'entrée de menu « Saisir un match » dans cet espace ?
///
/// # Pourquoi la question porte sur l'espace et non sur une compétition
///
/// Le menu est global à l'espace ; l'option vit sur une compétition. Il n'existe
/// pas de réponse par compétition à donner à un menu qui n'en désigne aucune.
///
/// La règle retenue est donc **une seule suffit** : dès qu'une compétition de
/// l'espace interdit le hors-calendrier, l'entrée disparaît pour tout le monde.
/// Un espace à trois compétitions dont une seule interdit perd l'entrée partout,
/// alors que la saisie resterait légitime dans les deux autres — assumé, parce
/// qu'un menu menant à un formulaire refusé une fois sur trois serait pire.
///
/// # « Une compétition » veut dire « sa saison en cours »
///
/// L'option est portée par la saison, comme les quatre autres réglages. Seule la
/// **dernière** saison de chaque compétition est consultée : sans cela, une
/// saison archivée qui interdisait ferait disparaître l'entrée pour toujours.
#[async_trait]
pub trait IHorsCalendrierPort: Send + Sync {
    /// `false` en cas d'échec de lecture, et c'est délibéré : un menu amputé par
    /// une panne de base est plus déroutant qu'un menu complet dont une entrée
    /// mènera à un refus. La garde serveur, elle, ne se relâche pas.
    async fn un_espace_interdit(&self, space_id: &str) -> bool;
}
