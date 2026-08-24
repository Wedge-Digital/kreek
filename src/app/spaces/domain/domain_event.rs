use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, EventId, SpaceId};
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::shared_kernel::identity::spaces_app_events::SpacesAppEvent;
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_tags::EventTag;
use crate::common::services::event_bus::event_tags::EventTagName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SpacesDomainEvent {
    SpaceCreated {
        event_id: EventId,
        created_by: CoachId,
        space_name: SpaceName,
        space_logo: CloudinaryImage,
        space_id: SpaceId,
    },
    UserInvitedInSpace {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
    },
    UserSubscribedToSpace {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
        space_profile: SpaceProfile,
    },
    UserPromotedToSpaceAdmin {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
    },
    /// Symétrique de la promotion. Deux événements plutôt qu'un portant le rôle
    /// cible : `grep UserDemotedToSpaceUser` répond à une question, tandis
    /// qu'un `UserRoleChanged` unique obligerait à lire les charges utiles.
    UserDemotedToSpaceUser {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
    },
    /// L'ajout d'un membre par un administrateur, **sans son consentement**.
    ///
    /// Un événement à lui plutôt qu'un champ ajouté à `UserSubscribedToSpace` :
    /// celui-ci est émis par l'adhésion spontanée, où `added_by` vaudrait le
    /// coach lui-même. Un champ qui ne veut pas dire la même chose selon
    /// l'émetteur ne se lit pas.
    ///
    /// `added_by` est là parce que l'opération se passe du consentement de la
    /// cible : une opération sans consentement doit dire qui l'a ordonnée.
    UserAddedToSpaceByAdmin {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
        profile: SpaceProfile,
        added_by: CoachId,
    },
    /// Le retrait d'un membre par un administrateur.
    ///
    /// Seul des trois événements d'appartenance à **franchir la frontière** :
    /// un coach retiré peut être administrateur d'une compétition de l'espace.
    /// Promotion et rétrogradation restent internes, le rôle étant relu en
    /// direct par `SpacePermissions` à chaque requête.
    UserUnsubscribedFromSpace {
        event_id: EventId,
        user_id: CoachId,
        space_id: SpaceId,
    },
    SpaceArchived {
        event_id: EventId,
        space_id: SpaceId,
    },
}

pub const SPACE_CREATED: &str = "SpaceCreated";

/// Discordant avec le nom de sa variante, et **volontairement conservé**.
///
/// Cette chaîne est dans le journal d'événements depuis l'origine. La changer
/// ne renommerait pas les lignes déjà écrites : elle en ferait des orphelines
/// qu'aucune constante ne désigne plus, et couperait en deux l'historique d'un
/// même fait.
///
/// Un type d'événement persisté est un identifiant public, pas un nom de
/// variable — on ne le corrige pas parce qu'il a mal vieilli.
pub const USER_SUBSCRIBED_TO_SPACE: &str = "UserRegisteredInSpace";

/// Valait `"UserRegisteredInSpace"` — la **même chaîne** que
/// `USER_SUBSCRIBED_TO_SPACE`. Deux événements distincts partageaient donc leur
/// type : tout listener qui filtre dessus les attrapait tous les deux.
///
/// Le défaut est resté latent parce que `UserInvitedInSpace` n'est émis nulle
/// part. Changer cette valeur-ci ne coupe aucun historique, pour la même
/// raison : aucune ligne n'a jamais été écrite sous ce nom.
pub const USER_INVITED_IN_SPACE: &str = "UserInvitedInSpace";

pub const USER_PROMOTED_TO_SPACE_ADMIN: &str = "UserPromotedToSpaceAdmin";
pub const USER_DEMOTED_TO_SPACE_USER: &str = "UserDemotedToSpaceUser";
pub const USER_ADDED_TO_SPACE_BY_ADMIN: &str = "UserAddedToSpaceByAdmin";
pub const USER_UNSUBSCRIBED_FROM_SPACE: &str = "UserUnsubscribedFromSpace";
pub const SPACE_ARCHIVED: &str = "SpaceArchived";

impl SpacesDomainEvent {
    pub fn to_app_event(&self) -> Option<SpacesAppEvent> {
        match self {
            SpacesDomainEvent::SpaceCreated {
                space_name,
                space_id,
                space_logo,
                created_by,
                ..
            } => Some(SpacesAppEvent::SpaceCreated {
                event_id: EventId::new(),
                space_id: *space_id,
                space_name: space_name.clone(),
                space_logo: space_logo.clone(),
                created_by: *created_by,
            }),
            SpacesDomainEvent::UserSubscribedToSpace {
                user_id,
                space_id,
                space_profile,
                ..
            } => Some(SpacesAppEvent::UserSubscribed {
                event_id: EventId::new(),
                user_id: user_id.clone(),
                space_id: space_id.clone(),
                space_profile: space_profile.clone(),
            }),
            // L'ajout par un administrateur franchit la frontière sous le
            // **même** app event que l'adhésion spontanée. Le domaine sépare les
            // deux faits — le journal doit les distinguer d'un `grep` — mais
            // l'extérieur n'a besoin que de l'effet : un coach est membre.
            SpacesDomainEvent::UserAddedToSpaceByAdmin {
                user_id,
                space_id,
                profile,
                ..
            } => Some(SpacesAppEvent::UserSubscribed {
                event_id: EventId::new(),
                user_id: *user_id,
                space_id: *space_id,
                space_profile: profile.clone(),
            }),
            // Le retrait franchit la frontière : un coach retiré peut être
            // administrateur d'une compétition de l'espace. L'app event existait
            // déjà dans l'enum, sans émetteur ni auditeur — on le réveille.
            SpacesDomainEvent::UserUnsubscribedFromSpace {
                user_id, space_id, ..
            } => Some(SpacesAppEvent::UserUnsubscribed {
                event_id: EventId::new(),
                user_id: *user_id,
                space_id: *space_id,
            }),
            // Promotion et rétrogradation ne franchissent pas : le rôle d'espace
            // est relu en direct par `SpacePermissions` à chaque requête, aucun
            // BC n'en cache de copie.
            _ => None,
        }
    }

    pub fn to_event_type(&self) -> &'static str {
        match self {
            Self::SpaceCreated { .. } => SPACE_CREATED,
            Self::UserInvitedInSpace { .. } => USER_INVITED_IN_SPACE,
            Self::UserSubscribedToSpace { .. } => USER_SUBSCRIBED_TO_SPACE,
            Self::UserPromotedToSpaceAdmin { .. } => USER_PROMOTED_TO_SPACE_ADMIN,
            Self::UserDemotedToSpaceUser { .. } => USER_DEMOTED_TO_SPACE_USER,
            Self::UserAddedToSpaceByAdmin { .. } => USER_ADDED_TO_SPACE_BY_ADMIN,
            Self::UserUnsubscribedFromSpace { .. } => USER_UNSUBSCRIBED_FROM_SPACE,
            Self::SpaceArchived { .. } => SPACE_ARCHIVED,
        }
    }

    fn space_id(&self) -> CoachId {
        match self {
            Self::SpaceCreated { space_id, .. } => *space_id,
            Self::UserInvitedInSpace { space_id, .. } => *space_id,
            Self::UserSubscribedToSpace { space_id, .. } => *space_id,
            Self::UserPromotedToSpaceAdmin { space_id, .. } => *space_id,
            Self::UserDemotedToSpaceUser { space_id, .. } => *space_id,
            Self::UserAddedToSpaceByAdmin { space_id, .. } => *space_id,
            Self::UserUnsubscribedFromSpace { space_id, .. } => *space_id,
            Self::SpaceArchived { space_id, .. } => *space_id,
        }
    }

    pub fn get_tags(&self) -> Vec<EventTag> {
        match self {
            Self::SpaceCreated { space_id, .. } => vec![EventTag {
                name: EventTagName::Space,
                value: space_id.to_string(),
            }],
            Self::UserInvitedInSpace {
                space_id, user_id, ..
            } => vec![
                EventTag {
                    name: EventTagName::Space,
                    value: space_id.to_string(),
                },
                EventTag {
                    name: EventTagName::User,
                    value: user_id.to_string(),
                },
            ],
            Self::UserSubscribedToSpace {
                space_id, user_id, ..
            } => vec![
                EventTag {
                    name: EventTagName::Space,
                    value: space_id.to_string(),
                },
                EventTag {
                    name: EventTagName::User,
                    value: user_id.to_string(),
                },
            ],
            Self::UserPromotedToSpaceAdmin {
                space_id, user_id, ..
            }
            | Self::UserDemotedToSpaceUser {
                space_id, user_id, ..
            }
            | Self::UserUnsubscribedFromSpace {
                space_id, user_id, ..
            }
            | Self::UserAddedToSpaceByAdmin {
                space_id, user_id, ..
            } => vec![
                EventTag {
                    name: EventTagName::Space,
                    value: space_id.to_string(),
                },
                EventTag {
                    name: EventTagName::User,
                    value: user_id.to_string(),
                },
            ],
            Self::SpaceArchived { space_id, .. } => vec![EventTag {
                name: EventTagName::Space,
                value: space_id.to_string(),
            }],
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: self.space_id().to_string(),
            event_type: self.to_event_type().parse().unwrap(),
            tags: serde_json::to_value(self.get_tags()).unwrap(),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::identity::ids::UserId;

    /// Les cinq variantes, chacune construite une fois.
    fn toutes() -> Vec<SpacesDomainEvent> {
        let space_id = SpaceId::new();
        let user_id = UserId::new();
        vec![
            SpacesDomainEvent::SpaceCreated {
                event_id: EventId::new(),
                created_by: user_id,
                space_name: SpaceName::try_new("Tribu Celtique").unwrap(),
                space_logo: CloudinaryImage::try_new(
                    "https://res.cloudinary.com/demo/kreek/tribu.png".to_string(),
                )
                .unwrap(),
                space_id,
            },
            SpacesDomainEvent::UserInvitedInSpace {
                event_id: EventId::new(),
                user_id,
                space_id,
            },
            SpacesDomainEvent::UserSubscribedToSpace {
                event_id: EventId::new(),
                user_id,
                space_id,
                space_profile: SpaceProfile::SpaceUser,
            },
            SpacesDomainEvent::UserPromotedToSpaceAdmin {
                event_id: EventId::new(),
                user_id,
                space_id,
            },
            SpacesDomainEvent::UserDemotedToSpaceUser {
                event_id: EventId::new(),
                user_id,
                space_id,
            },
            SpacesDomainEvent::UserUnsubscribedFromSpace {
                event_id: EventId::new(),
                user_id,
                space_id,
            },
            SpacesDomainEvent::UserAddedToSpaceByAdmin {
                event_id: EventId::new(),
                user_id,
                space_id,
                profile: SpaceProfile::SpaceUser,
                added_by: user_id,
            },
            SpacesDomainEvent::SpaceArchived {
                event_id: EventId::new(),
                space_id,
            },
        ]
    }

    /// Force à compléter `toutes()` quand une variante apparaît.
    ///
    /// Le test de distinction énumère les variantes **à la main**, et il a
    /// silencieusement dérivé : trois variantes ajoutées après la carte 364 n'y
    /// figuraient pas, et le verrou ne couvrait plus que cinq cas sur huit. Un
    /// test qui énumère ne sait pas qu'il est incomplet.
    ///
    /// Ce `match` exhaustif, lui, ne compile plus dès qu'une variante apparaît.
    /// Le compilateur amène alors ici, et ce commentaire amène à `toutes()`.
    #[allow(dead_code)]
    fn _completude(e: &SpacesDomainEvent) {
        match e {
            SpacesDomainEvent::SpaceCreated { .. }
            | SpacesDomainEvent::UserInvitedInSpace { .. }
            | SpacesDomainEvent::UserSubscribedToSpace { .. }
            | SpacesDomainEvent::UserPromotedToSpaceAdmin { .. }
            | SpacesDomainEvent::UserDemotedToSpaceUser { .. }
            | SpacesDomainEvent::UserAddedToSpaceByAdmin { .. }
            | SpacesDomainEvent::UserUnsubscribedFromSpace { .. }
            | SpacesDomainEvent::SpaceArchived { .. } => {}
        }
    }

    /// Le test qui aurait attrapé le défaut, et qui attrapera le prochain.
    ///
    /// Vérifier qu'une variante rend *sa* chaîne ne suffit pas : `UserInvited`
    /// et `UserSubscribed` rendaient chacune la sienne, et c'était la même. Ce
    /// qu'il faut vérifier est une propriété de l'ensemble — un type par fait.
    ///
    /// La liste vient de `toutes()`, que `_completude` oblige à tenir à jour.
    #[test]
    fn chaque_variante_rend_un_type_distinct() {
        let types: Vec<&str> = toutes().iter().map(|e| e.to_event_type()).collect();
        let distincts: std::collections::HashSet<&str> = types.iter().copied().collect();

        assert_eq!(
            distincts.len(),
            types.len(),
            "deux variantes partagent leur type d'événement : {types:?}"
        );
    }

    /// Ce que le journal contient déjà ne doit pas changer de nom.
    ///
    /// Cinq lignes y sont écrites sous `UserRegisteredInSpace`, toutes des
    /// souscriptions. Renommer cette valeur les rendrait orphelines.
    #[test]
    fn la_souscription_garde_le_type_present_dans_le_journal() {
        let event = SpacesDomainEvent::UserSubscribedToSpace {
            event_id: EventId::new(),
            user_id: UserId::new(),
            space_id: SpaceId::new(),
            space_profile: SpaceProfile::SpaceUser,
        };
        assert_eq!(event.to_event_type(), "UserRegisteredInSpace");
    }

    /// L'invitation prend le nom qu'elle aurait toujours dû avoir. Aucune ligne
    /// n'a jamais été écrite sous l'ancien, l'événement n'étant émis nulle part.
    #[test]
    fn l_invitation_a_son_propre_type() {
        let event = SpacesDomainEvent::UserInvitedInSpace {
            event_id: EventId::new(),
            user_id: UserId::new(),
            space_id: SpaceId::new(),
        };
        assert_eq!(event.to_event_type(), "UserInvitedInSpace");
    }
}
