//! Les quatre gabarits d'e-mail de compétition, et leurs contextes de rendu.
//!
//! # Pourquoi dans `io/`
//!
//! Rendre un e-mail est de l'IO, au même titre que rendre une page. Le use case
//! d'expédition (carte 339) remplit ces structs ; il n'écrit pas de HTML.
//!
//! # Contraintes d'e-mail, pas de page web
//!
//! - le logo est en URL absolue, **jamais** un `data:` URI — Gmail les retire ;
//! - `width` et `height` sont des **attributs HTML** : Outlook ignore le CSS de
//!   dimension ;
//! - aucune feuille externe, tout le style est en ligne ;
//! - `app_url` **porte son schéma**, et arrive déjà normalisé par
//!   `AppConfig::app_url()` : rien n'est à recoller ici.

use askama::Template;

/// Ce que le coach joue cette journée, côté rendu.
///
/// Un enum et non un `Vec` : un `Vec` vide se rendrait **en silence**, et la
/// ligne « tu ne joues pas » que R4 impose disparaîtrait sans que rien ne
/// proteste. Le gabarit doit traiter les deux branches jusqu'au dernier mètre.
pub enum ParticipationVm {
    NotPlaying,
    Playing(Vec<FixtureVm>),
}

pub struct FixtureVm {
    /// L'équipe du coach dans cet appariement — elle distingue ses deux lignes
    /// quand il en aligne deux le même jour.
    pub team_name: String,
    pub home_team: String,
    pub away_team: String,
}

/// Veille de journée. **Deux axes de variation indépendants**, et quatre
/// combinaisons toutes atteignables : une journée à date fixe pour un coach qui
/// ne joue pas est ordinaire. Les confondre en une condition amputerait
/// l'e-mail d'un quart des destinataires.
#[derive(Template)]
#[template(path = "emails/fr_FR/competition_round_eve.html")]
pub struct RoundEveEmail {
    pub app_url: String,
    pub coach_name: String,
    pub competition_name: String,
    pub competition_url: String,
    pub round_name: String,
    pub date_start: String,
    /// `None` pour une journée à date fixe : la ligne « Clôture » disparaît et
    /// « Ouverture » devient « Se tient le ».
    pub date_end: Option<String>,
    pub participation: ParticipationVm,
}

/// Fin de journée imminente. Ne part que sur une journée à fenêtre temporelle —
/// une date fixe n'a pas de fin à anticiper — mais à **tous** les inscrits (R4).
#[derive(Template)]
#[template(path = "emails/fr_FR/competition_round_closing.html")]
pub struct RoundClosingEmail {
    pub app_url: String,
    pub coach_name: String,
    pub competition_name: String,
    pub competition_url: String,
    pub round_name: String,
    pub date_end: String,
    pub participation: ParticipationVm,
}

#[derive(Template)]
#[template(path = "emails/fr_FR/competition_registration_open.html")]
pub struct RegistrationOpenEmail {
    pub app_url: String,
    pub coach_name: String,
    pub admin_name: String,
    pub space_name: String,
    pub competition_name: String,
    pub season_name: String,
    pub competition_url: String,
    pub registration_deadline: String,
}

#[derive(Template)]
#[template(path = "emails/fr_FR/competition_registration_deadline.html")]
pub struct RegistrationDeadlineEmail {
    pub app_url: String,
    pub coach_name: String,
    pub admin_name: String,
    // Pas de `space_name` : la maquette de la relance ne nomme pas l'espace,
    // contrairement à celle de l'ouverture. La phase 4 de la spec le listait —
    // la maquette, validée en phase 1, fait foi. Un champ qu'aucun gabarit ne
    // lit se remplit sans fin de valeurs que personne ne voit.
    pub competition_name: String,
    pub season_name: String,
    pub competition_url: String,
    pub registration_deadline: String,
    pub remaining_slots: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(equipe: &str, adversaire: &str) -> FixtureVm {
        FixtureVm {
            team_name: equipe.to_string(),
            home_team: equipe.to_string(),
            away_team: adversaire.to_string(),
        }
    }

    fn veille(date_end: Option<&str>, participation: ParticipationVm) -> String {
        RoundEveEmail {
            app_url: "https://kreek.example".into(),
            coach_name: "Alice".into(),
            competition_name: "Ligue de Fer".into(),
            competition_url: "https://kreek.example/app/s/competitions/c/x".into(),
            round_name: "Journée 3".into(),
            date_start: "11/09/2026".into(),
            date_end: date_end.map(str::to_string),
            participation,
        }
        .render()
        .expect("le gabarit doit se rendre")
    }

    // ── Les quatre combinaisons de la veille ─────────────────────────────────

    #[test]
    fn fenetre_temporelle_et_match_affiche_la_cloture_et_l_adversaire() {
        let h = veille(
            Some("18/09/2026"),
            ParticipationVm::Playing(vec![fixture("Les Uns", "Les Trois")]),
        );

        assert!(h.contains("Clôture"));
        assert!(h.contains("18/09/2026"));
        assert!(h.contains("Les Trois"), "l'adversaire doit apparaître");
        assert!(h.contains("Ton match"));
    }

    #[test]
    fn date_fixe_et_match_supprime_la_cloture_et_change_le_libelle() {
        let h = veille(
            None,
            ParticipationVm::Playing(vec![fixture("Les Uns", "Les Trois")]),
        );

        assert!(!h.contains("Clôture"), "une date fixe n'a pas de fin");
        assert!(h.contains("Se tient le"));
        assert!(h.contains("Les Trois"));
    }

    /// R4 : tous les inscrits reçoivent l'e-mail, y compris ceux qui ne jouent
    /// pas. C'est une information, pas une absence.
    #[test]
    fn fenetre_temporelle_sans_match_dit_qu_on_ne_joue_pas() {
        let h = veille(Some("18/09/2026"), ParticipationVm::NotPlaying);

        assert!(h.contains("Tu ne joues pas"));
        assert!(h.contains("Clôture"));
        assert!(!h.contains("Ton match"));
    }

    #[test]
    fn date_fixe_sans_match_est_une_combinaison_ordinaire() {
        let h = veille(None, ParticipationVm::NotPlaying);

        assert!(h.contains("Tu ne joues pas"));
        assert!(!h.contains("Clôture"));
        assert!(h.contains("Se tient le"));
    }

    #[test]
    fn deux_equipes_donnent_deux_lignes_et_un_titre_au_pluriel() {
        let h = veille(
            Some("18/09/2026"),
            ParticipationVm::Playing(vec![
                fixture("Les Uns", "Les Trois"),
                fixture("Les Deux", "Les Quatre"),
            ]),
        );

        assert!(h.contains("Tes matchs"));
        assert!(h.contains("Les Trois") && h.contains("Les Quatre"));
    }

    // ── Contraintes d'e-mail ─────────────────────────────────────────────────

    #[test]
    fn le_logo_est_une_url_absolue_avec_ses_dimensions_en_attributs() {
        let h = veille(None, ParticipationVm::NotPlaying);

        assert!(h.contains("https://kreek.example/static/img/email-logo.png"));
        assert!(!h.contains("data:"), "Gmail retire les data: URI");
        assert!(h.contains("width=\"200\"") && h.contains("height=\"81\""));
    }

    // ── Le contrôle qui a manqué une fois ────────────────────────────────────

    /// Une substitution a mangé `.header-title` et `.header-sub` en phase 1,
    /// laissant un texte sombre sur fond sombre. Le contrôle était prévu « à la
    /// main » ; à la main, il ne se refera pas.
    ///
    /// Les classes ne servant qu'à `<style>` (pseudo-sélecteurs, descendants)
    /// sont couvertes : on ne compare que les noms, pas les sélecteurs entiers.
    fn classes_sans_regle(html: &str) -> Vec<String> {
        let style = html
            .split("<style>")
            .nth(1)
            .and_then(|s| s.split("</style>").next())
            .unwrap_or_default();
        let definies: std::collections::HashSet<String> = style
            .split('.')
            .skip(1)
            .filter_map(|s| {
                let nom: String = s
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                (!nom.is_empty()).then_some(nom)
            })
            .collect();

        let corps = html.split("</style>").nth(1).unwrap_or_default();
        let mut manquantes = Vec::new();
        for morceau in corps.split("class=\"").skip(1) {
            let Some(valeur) = morceau.split('"').next() else {
                continue;
            };
            for classe in valeur.split_whitespace() {
                if !definies.contains(classe) {
                    manquantes.push(classe.to_string());
                }
            }
        }
        manquantes.sort();
        manquantes.dedup();
        manquantes
    }

    /// **Le test qui manquait, et dont l'absence a coûté cher.**
    ///
    /// Trois variables partaient vides — `admin_name`, `space_name`,
    /// `remaining_slots` — parce que le use case les câblait en `String::new()`
    /// en attendant de les remplir. L'e-mail d'ouverture disait
    /// « **** t'invite à participer », et il est parti comme ça.
    ///
    /// Les autres tests vérifiaient qu'une donnée **présente** s'affiche ;
    /// aucun ne vérifiait qu'aucune donnée ne manque. C'est le pendant, côté
    /// données, du contrôle des classes orphelines.
    #[test]
    fn aucune_variable_ne_se_rend_vide() {
        // Chaque champ porte une valeur reconnaissable : si l'une n'apparaît
        // pas dans le rendu, c'est que le gabarit ne la lit pas — ou qu'un
        // appelant la laisserait vide sans qu'on le voie.
        let ouverture = RegistrationOpenEmail {
            app_url: "https://kreek.example".into(),
            coach_name: "VAL-coach".into(),
            admin_name: "VAL-admin".into(),
            space_name: "VAL-espace".into(),
            competition_name: "VAL-competition".into(),
            season_name: "VAL-saison".into(),
            competition_url: "https://kreek.example/VAL-url".into(),
            registration_deadline: "VAL-deadline".into(),
        }
        .render()
        .unwrap();
        for attendu in [
            "VAL-coach",
            "VAL-admin",
            "VAL-espace",
            "VAL-competition",
            "VAL-url",
        ] {
            assert!(ouverture.contains(attendu), "ouverture : {attendu} absent");
        }

        let limite = RegistrationDeadlineEmail {
            app_url: "https://kreek.example".into(),
            coach_name: "VAL-coach".into(),
            admin_name: "VAL-admin".into(),
            competition_name: "VAL-competition".into(),
            season_name: "VAL-saison".into(),
            competition_url: "https://kreek.example/VAL-url".into(),
            registration_deadline: "VAL-deadline".into(),
            remaining_slots: "VAL-places".into(),
        }
        .render()
        .unwrap();
        for attendu in [
            "VAL-coach",
            "VAL-admin",
            "VAL-competition",
            "VAL-deadline",
            "VAL-places",
        ] {
            assert!(limite.contains(attendu), "date limite : {attendu} absent");
        }

        let veille = RoundEveEmail {
            app_url: "https://kreek.example".into(),
            coach_name: "VAL-coach".into(),
            competition_name: "VAL-competition".into(),
            competition_url: "https://kreek.example/VAL-url".into(),
            round_name: "VAL-journee".into(),
            date_start: "VAL-debut".into(),
            date_end: Some("VAL-fin".into()),
            participation: ParticipationVm::Playing(vec![FixtureVm {
                team_name: "VAL-mon-equipe".into(),
                home_team: "VAL-domicile".into(),
                away_team: "VAL-exterieur".into(),
            }]),
        }
        .render()
        .unwrap();
        for attendu in [
            "VAL-coach",
            "VAL-competition",
            "VAL-url",
            "VAL-journee",
            "VAL-debut",
            "VAL-fin",
            "VAL-mon-equipe",
            "VAL-domicile",
            "VAL-exterieur",
        ] {
            assert!(veille.contains(attendu), "veille : {attendu} absent");
        }
    }

    #[test]
    fn aucune_classe_utilisee_n_a_perdu_sa_regle() {
        let rendus = [
            veille(
                Some("18/09/2026"),
                ParticipationVm::Playing(vec![fixture("Les Uns", "Les Trois")]),
            ),
            veille(None, ParticipationVm::NotPlaying),
            RoundClosingEmail {
                app_url: "https://kreek.example".into(),
                coach_name: "Alice".into(),
                competition_name: "Ligue de Fer".into(),
                competition_url: "https://kreek.example/x".into(),
                round_name: "Journée 3".into(),
                date_end: "18/09/2026".into(),
                participation: ParticipationVm::Playing(vec![fixture("Les Uns", "Les Trois")]),
            }
            .render()
            .unwrap(),
            RegistrationOpenEmail {
                app_url: "https://kreek.example".into(),
                coach_name: "Alice".into(),
                admin_name: "Bob".into(),
                space_name: "Espace".into(),
                competition_name: "Ligue de Fer".into(),
                season_name: "Saison 1".into(),
                competition_url: "https://kreek.example/x".into(),
                registration_deadline: "20/09/2026".into(),
            }
            .render()
            .unwrap(),
            RegistrationDeadlineEmail {
                app_url: "https://kreek.example".into(),
                coach_name: "Alice".into(),
                admin_name: "Bob".into(),
                competition_name: "Ligue de Fer".into(),
                season_name: "Saison 1".into(),
                competition_url: "https://kreek.example/x".into(),
                registration_deadline: "20/09/2026".into(),
                remaining_slots: "3".into(),
            }
            .render()
            .unwrap(),
        ];

        for (i, html) in rendus.iter().enumerate() {
            let manquantes = classes_sans_regle(html);
            assert!(
                manquantes.is_empty(),
                "gabarit {i} : classes sans règle — {manquantes:?}"
            );
        }
    }
}
