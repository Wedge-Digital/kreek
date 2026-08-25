//! Ce qui est dû aujourd'hui, et rien d'autre.
//!
//! # La signature tient R9 à elle seule
//!
//! `due_today()` n'a **aucun accès au journal d'envois**. Elle ne peut donc pas
//! poser la question « qu'est-ce qui n'est pas parti hier ? », faute d'avoir la
//! donnée pour y répondre — c'est une garantie de structure, pas de discipline.
//! Ne pas lui ajouter de paramètre qui l'y autoriserait.
//!
//! # Elle ne fusionne pas avec `applicability()`
//!
//! Les deux se ressemblent assez pour qu'on soit tenté, et répondent à deux
//! questions différentes :
//!
//! | | Question | Entrées |
//! |---|---|---|
//! | `applicability()` | « cela peut-il arriver un jour ? » | la **structure** |
//! | `due_today()` | « est-ce dû aujourd'hui ? » | les **journées persistées** |
//!
//! La structure décrit ce qui a été *voulu*, les journées ce qui *existe*. Les
//! fusionner obligerait l'écran de réglage à charger les journées, et
//! l'ordonnanceur à charger la structure, pour rien. Leurs verdicts restent
//! cohérents sans code commun : ils viennent des mêmes faits.

use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
use crate::app::competitions::domain::match_day::{MatchDay, MatchDayName, MatchDayType, Pairing};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::MatchId;
use time::macros::format_description;
use time::Date;

/// Les trois décalages sont des **constantes de domaine**, pas de la
/// configuration.
///
/// La maquette de date limite écrit « Plus que trois jours » en toutes lettres,
/// et celle de fin de journée annonce sa fenêtre. Un réglage laisserait le
/// nombre et le texte diverger sans que rien ne le signale — un e-mail affirmant
/// « plus que trois jours » cinq jours avant l'échéance est pire que pas
/// d'e-mail du tout.
///
/// Si ces valeurs devaient un jour être réglables, le texte devrait le devenir
/// dans le même mouvement. Les changer ensemble, ou pas du tout.
const EVE_OFFSET_DAYS: i64 = 1;
const CLOSING_OFFSET_DAYS: i64 = 2;
const DEADLINE_OFFSET_DAYS: i64 = 3;

/// Une notification à envoyer, et le contexte minimal qui la qualifie.
///
/// `RegistrationOpen` n'y figure pas : elle se déclenche sur un fait — la saison
/// s'ouvre — et non sur une date à comparer à aujourd'hui (R11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DueNotification {
    RoundEve { round: RoundRef },
    RoundClosing { round: RoundRef },
    RegistrationDeadline { deadline: DateString },
}

/// `day_type` voyage parce que le gabarit en dépend : une journée à date fixe
/// n'a pas de ligne « clôture ». `MatchDayType::Rest` ne peut pas y apparaître,
/// `due_today()` l'ignorant.
///
/// `pairings` voyage aussi, et ce n'est pas dans la spec d'origine : la
/// résolution des destinataires (carte 337) doit croiser les appariements de la
/// journée avec les équipes du coach, et rien ne les lui apportait. Ils sont
/// **déjà chargés** ici — `MatchDay` les porte — donc les recopier coûte moins
/// qu'un port de lecture supplémentaire pour une donnée déjà en main.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundRef {
    pub round_id: MatchId,
    pub round_name: MatchDayName,
    pub date_start: DateString,
    pub date_end: Option<DateString>,
    pub day_type: MatchDayType,
    pub pairings: Vec<Pairing>,
}

/// Les trois dates que le cron doit interroger pour un jour donné.
///
/// **Elle existe pour que les décalages n'aient qu'une source.** Le SQL du cron
/// sélectionne les saisons « ayant une journée à la date donnée » ; `due_today()`
/// compare, elle, à `today + n`. Si l'appelant calculait ces dates lui-même, les
/// deux finiraient par diverger — et le symptôme serait un cron qui ne trouve
/// jamais rien, sans la moindre erreur.
///
/// C'est exactement ce qui s'est produit à l'écriture de la carte 340.
pub struct Fenetres {
    /// Les journées qui démarrent, pour la veille.
    pub round_eve: DateString,
    /// Les journées à fenêtre qui closent, pour l'alerte de clôture.
    pub round_closing: DateString,
    /// Les dates limites d'inscription approchant.
    pub registration_deadline: DateString,
}

pub fn fenetres(today: &DateString) -> Option<Fenetres> {
    let jour = analyser(today)?;
    let vers = |n| {
        decale(jour, n)
            .format(format_description!("[year]-[month]-[day]"))
            .ok()
            .and_then(|s| DateString::try_new(&s).ok())
    };
    Some(Fenetres {
        round_eve: vers(EVE_OFFSET_DAYS)?,
        round_closing: vers(CLOSING_OFFSET_DAYS)?,
        registration_deadline: vers(DEADLINE_OFFSET_DAYS)?,
    })
}

/// Ne retourne pas de `Result` : une date illisible ne peut pas venir du
/// domaine, `DateString` validant son format à la construction. Le seul point de
/// défaillance est l'analyse de `today`, fourni par la CLI — traité au bord du
/// système, pas ici.
pub fn due_today(
    today: &DateString,
    match_days: &[MatchDay],
    invitations: Option<&CompetitionInvitations>,
    settings: &CompetitionNotifications,
) -> Vec<DueNotification> {
    let Some(jour) = analyser(today) else {
        return Vec::new();
    };

    let mut dues = Vec::new();
    for j in match_days {
        if let Some(d) = veille(j, jour, settings) {
            dues.push(d);
        }
        if let Some(d) = cloture(j, jour, settings) {
            dues.push(d);
        }
    }
    if let Some(d) = date_limite(invitations, jour, settings) {
        dues.push(d);
    }
    dues
}

fn veille(
    j: &MatchDay,
    today: Date,
    settings: &CompetitionNotifications,
) -> Option<DueNotification> {
    if !settings.round_eve.0 || j.day_type == MatchDayType::Rest {
        return None;
    }
    let debut = date_utile(j.date_start.as_ref())?;
    if analyser(debut)? != decale(today, EVE_OFFSET_DAYS) {
        return None;
    }
    Some(DueNotification::RoundEve {
        round: reference(j, debut),
    })
}

/// Exige le type `TimeFrame`, **pas** simplement une `date_end` non nulle. Les
/// deux conditions coïncident aujourd'hui ; s'appuyer sur la seconde ferait
/// dépendre une règle métier d'un invariant de persistance que rien ne garantit.
fn cloture(
    j: &MatchDay,
    today: Date,
    settings: &CompetitionNotifications,
) -> Option<DueNotification> {
    if !settings.round_closing.0 || j.day_type != MatchDayType::TimeFrame {
        return None;
    }
    let fin = date_utile(j.date_end.as_ref())?;
    if analyser(fin)? != decale(today, CLOSING_OFFSET_DAYS) {
        return None;
    }
    let debut = date_utile(j.date_start.as_ref())?;
    Some(DueNotification::RoundClosing {
        round: reference(j, debut),
    })
}

fn date_limite(
    invitations: Option<&CompetitionInvitations>,
    today: Date,
    settings: &CompetitionNotifications,
) -> Option<DueNotification> {
    if !settings.registration_deadline.0 {
        return None;
    }
    let brut = invitations?.registration_deadline.as_deref()?;
    if brut.trim().is_empty() {
        return None;
    }
    let limite = DateString::try_new(brut).ok()?;
    if analyser(&limite)? != decale(today, DEADLINE_OFFSET_DAYS) {
        return None;
    }
    Some(DueNotification::RegistrationDeadline { deadline: limite })
}

fn reference(j: &MatchDay, debut: &DateString) -> RoundRef {
    RoundRef {
        round_id: j.id.clone(),
        round_name: j.name.clone(),
        date_start: debut.clone(),
        // `date_end` **seulement** pour une fenêtre temporelle, et c'est le
        // `day_type` qui commande — pas la présence d'une date en base.
        //
        // Le magicien persiste `date_end = date_start` pour une journée à date
        // fixe. S'y fier ferait afficher une ligne « Clôture » sur une journée
        // qui n'a rien à clore, et le libellé « Ouverture » là où la maquette
        // dit « Se tient le ». C'est la même règle que pour `RoundClosing` :
        // une décision d'affichage ne se déduit pas d'un artefact de
        // persistance.
        date_end: match j.day_type {
            MatchDayType::TimeFrame => date_utile(j.date_end.as_ref()).cloned(),
            _ => None,
        },
        day_type: j.day_type.clone(),
        pairings: j.pairings.clone(),
    }
}

fn decale(today: Date, jours: i64) -> Date {
    today.saturating_add(time::Duration::days(jours))
}

/// La chaîne vide n'est pas une date. `DateString` est validée
/// `^(?:\d{4}-\d{2}-\d{2})?$`, donc `""` passe : elle doit être traitée comme
/// absente, au même titre que `None`.
fn date_utile(d: Option<&DateString>) -> Option<&DateString> {
    d.filter(|v| !v.as_ref().trim().is_empty())
}

fn analyser(d: &DateString) -> Option<Date> {
    Date::parse(
        d.as_ref().trim(),
        format_description!("[year]-[month]-[day]"),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayName;
    use crate::app::competitions::domain::match_day::MatchDayPosition;

    const AUJOURDHUI: &str = "2026-09-10";

    fn d(s: &str) -> DateString {
        DateString::try_new(s).unwrap()
    }

    fn journee(day_type: MatchDayType, debut: Option<&str>, fin: Option<&str>) -> MatchDay {
        MatchDay {
            id: MatchId::try_new("01KZVCKDG19DXZHJA295WSJGMX").unwrap(),
            season_id: crate::app::shared_kernel::bloodbowl::ids::SeasonId::try_new(
                "01KZVCKDG19DXZHJA295WSJGMV",
            )
            .unwrap(),
            name: MatchDayName::try_new("Journée 3").unwrap(),
            day_type,
            date_start: debut.map(d),
            date_end: fin.map(d),
            position: MatchDayPosition::try_new(3).unwrap(),
            pairings: Vec::new(),
        }
    }

    fn tout_allume() -> CompetitionNotifications {
        CompetitionNotifications::default()
    }

    fn invitations(deadline: Option<&str>) -> CompetitionInvitations {
        let v = deadline.map_or("null".to_string(), |x| format!("\"{x}\""));
        serde_json::from_str(&format!(
            r#"{{"max_participants":null,"registration_deadline":{v}}}"#
        ))
        .unwrap()
    }

    fn du(journees: &[MatchDay], inv: Option<&CompetitionInvitations>) -> Vec<DueNotification> {
        due_today(&d(AUJOURDHUI), journees, inv, &tout_allume())
    }

    // ── Veille de journée ────────────────────────────────────────────────────

    #[test]
    fn une_journee_demarrant_demain_est_annoncee() {
        let dues = du(
            &[journee(MatchDayType::FixedDate, Some("2026-09-11"), None)],
            None,
        );

        assert!(matches!(
            dues.as_slice(),
            [DueNotification::RoundEve { .. }]
        ));
    }

    #[test]
    fn le_reglage_eteint_supprime_l_annonce() {
        let mut eteint = tout_allume();
        eteint.round_eve =
            crate::app::competitions::domain::competition_notifications::NotifyRoundEve(false);

        let dues = due_today(
            &d(AUJOURDHUI),
            &[journee(MatchDayType::FixedDate, Some("2026-09-11"), None)],
            None,
            &eteint,
        );

        assert!(dues.is_empty());
    }

    #[test]
    fn une_journee_demarrant_apres_demain_n_est_pas_annoncee() {
        // Pas de fenêtre glissante : la veille, c'est la veille.
        let dues = du(
            &[journee(MatchDayType::FixedDate, Some("2026-09-12"), None)],
            None,
        );

        assert!(dues.is_empty());
    }

    /// **Le garde-fou unitaire de R9.** C'est le seul endroit où la règle se
    /// vérifie sans monter de base : si quelqu'un ajoute un jour une tolérance
    /// de rattrapage, ce test rougit avant tout le reste.
    #[test]
    fn une_journee_ayant_demarre_hier_n_est_jamais_rattrapee() {
        let dues = du(
            &[journee(MatchDayType::FixedDate, Some("2026-09-09"), None)],
            None,
        );

        assert!(dues.is_empty(), "aucun regard en arrière — R9");
    }

    #[test]
    fn une_journee_de_repos_n_a_rien_a_annoncer() {
        let dues = du(
            &[journee(MatchDayType::Rest, Some("2026-09-11"), None)],
            None,
        );

        assert!(dues.is_empty());
    }

    #[test]
    fn une_date_de_debut_vide_vaut_une_date_absente() {
        let dues = du(&[journee(MatchDayType::FixedDate, Some(""), None)], None);

        assert!(dues.is_empty());
    }

    // ── Fin de journée ───────────────────────────────────────────────────────

    #[test]
    fn une_fenetre_cloturant_dans_deux_jours_est_annoncee() {
        let dues = du(
            &[journee(
                MatchDayType::TimeFrame,
                Some("2026-09-05"),
                Some("2026-09-12"),
            )],
            None,
        );

        assert!(matches!(
            dues.as_slice(),
            [DueNotification::RoundClosing { .. }]
        ));
    }

    /// Le piège que la spec nomme : c'est le **type** qui commande, pas la
    /// présence d'une `date_end`. Ici la date est bonne et le type ne l'est pas.
    #[test]
    fn une_date_fixe_ne_cloture_pas_meme_avec_une_date_de_fin() {
        let dues = du(
            &[journee(
                MatchDayType::FixedDate,
                Some("2026-09-05"),
                Some("2026-09-12"),
            )],
            None,
        );

        assert!(dues.is_empty(), "seul TimeFrame a une fenêtre à clore");
    }

    // ── Date limite d'inscription ────────────────────────────────────────────

    #[test]
    fn une_date_limite_dans_trois_jours_est_annoncee() {
        let inv = invitations(Some("2026-09-13"));
        let dues = du(&[], Some(&inv));

        assert!(matches!(
            dues.as_slice(),
            [DueNotification::RegistrationDeadline { .. }]
        ));
    }

    #[test]
    fn sans_invitations_aucune_date_limite_n_est_annoncee() {
        assert!(du(&[], None).is_empty());
    }

    #[test]
    fn une_date_limite_vide_vaut_une_date_limite_absente() {
        let inv = invitations(Some(""));

        assert!(du(&[], Some(&inv)).is_empty());
    }

    /// Le magicien persiste `date_end = date_start` sur une journée à date fixe.
    /// Si l'e-mail s'y fiait, il annoncerait une clôture à une journée qui n'a
    /// rien à clore — constaté en lisant un envoi réel, pas en relisant le code.
    #[test]
    fn une_date_fixe_ne_porte_pas_de_date_de_fin_meme_si_la_base_en_a_une() {
        let j = journee(
            MatchDayType::FixedDate,
            Some("2026-09-11"),
            Some("2026-09-11"),
        );

        let dues = du(&[j], None);

        match dues.as_slice() {
            [DueNotification::RoundEve { round }] => assert_eq!(round.date_end, None),
            autre => panic!("attendu une veille, reçu {autre:?}"),
        }
    }

    #[test]
    fn une_fenetre_temporelle_garde_sa_date_de_fin() {
        let j = journee(
            MatchDayType::TimeFrame,
            Some("2026-09-11"),
            Some("2026-09-18"),
        );

        let dues = du(&[j], None);

        match dues.as_slice() {
            [DueNotification::RoundEve { round }] => {
                assert_eq!(
                    round.date_end.as_ref().map(|d| d.as_ref()),
                    Some("2026-09-18")
                )
            }
            autre => panic!("attendu une veille, reçu {autre:?}"),
        }
    }

    // ── Les fenêtres du cron ─────────────────────────────────────────────────

    /// **Le test qui manquait.** Le SQL du cron sélectionne les saisons ayant
    /// une journée **à la date donnée** ; `due_today()` compare à `today + n`.
    /// Passer `today` aux deux fait que le cron ne trouve **jamais rien**, sans
    /// la moindre erreur — c'est le défaut qui n'est apparu qu'en lançant la
    /// commande à la main, à l'écriture de la carte 340.
    ///
    /// Ce test lie les deux : si un décalage change ici, il change partout.
    #[test]
    fn les_fenetres_du_cron_suivent_les_memes_decalages_que_due_today() {
        let f = fenetres(&d(AUJOURDHUI)).expect("2026-09-10 est lisible");

        assert_eq!(f.round_eve.as_ref(), "2026-09-11");
        assert_eq!(f.round_closing.as_ref(), "2026-09-12");
        assert_eq!(f.registration_deadline.as_ref(), "2026-09-13");
    }

    /// La cohérence, prouvée plutôt qu'affirmée : la journée trouvée par la
    /// fenêtre de la veille est bien celle que `due_today()` annonce.
    #[test]
    fn une_journee_a_la_date_de_la_fenetre_est_bien_annoncee() {
        let f = fenetres(&d(AUJOURDHUI)).unwrap();
        let j = journee(MatchDayType::FixedDate, Some(f.round_eve.as_ref()), None);

        let dues = du(&[j], None);

        assert!(matches!(
            dues.as_slice(),
            [DueNotification::RoundEve { .. }]
        ));
    }

    #[test]
    fn une_date_illisible_ne_donne_aucune_fenetre() {
        assert!(fenetres(&d("")).is_none());
    }

    // ── Deux notifications le même jour ──────────────────────────────────────

    /// Acceptées telles quelles : elles ne disent pas la même chose, et les
    /// fusionner demanderait une clé d'idempotence composite et un cinquième
    /// gabarit, pour un cas peu fréquent.
    #[test]
    fn une_journee_qui_demarre_et_une_autre_qui_cloture_font_deux_notifications() {
        let dues = du(
            &[
                journee(MatchDayType::FixedDate, Some("2026-09-11"), None),
                journee(
                    MatchDayType::TimeFrame,
                    Some("2026-09-05"),
                    Some("2026-09-12"),
                ),
            ],
            None,
        );

        assert_eq!(dues.len(), 2);
        assert!(dues
            .iter()
            .any(|x| matches!(x, DueNotification::RoundEve { .. })));
        assert!(dues
            .iter()
            .any(|x| matches!(x, DueNotification::RoundClosing { .. })));
    }
}
