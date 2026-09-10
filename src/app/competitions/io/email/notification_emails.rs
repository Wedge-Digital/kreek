//! Les six gabarits d'e-mail de compétition, et leurs contextes de rendu.
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

/// Une équipe engagée, et sa paire de liens.
///
/// R1 — la réponse porte sur l'équipe, jamais sur le coach. Un coach qui engage
/// deux équipes reçoit **un** e-mail à deux paires de boutons : un e-mail par
/// équipe multiplierait les messages pour la même soirée, et il ne saurait pas
/// lequel il a déjà traité.
///
/// **Les deux URL arrivent construites.** Les composer ici mettrait la forme du
/// lien dans du HTML d'e-mail, hors de portée de tout test.
pub struct EquipeLigneVm {
    pub team_name: String,
    pub yes_url: String,
    pub no_url: String,
}

/// L'ouverture d'une campagne de présence.
///
/// `equipes` est un `Vec` même pour un coach à une seule équipe : un champ
/// scalaire aurait obligé à un second gabarit dès la première ligue un peu
/// vivante, et le gabarit rend la même boucle dans les deux cas.
///
/// `date_start` + `date_end` et non un libellé composé : c'est la forme des
/// quatre e-mails plus anciens. La carte 526 annonçait un `round_dates` unique —
/// le suivre aurait donné à ce seul e-mail une mise en forme que les autres
/// n'ont pas.
#[derive(Template)]
#[template(path = "emails/fr_FR/competition_presence_survey.html")]
pub struct PresenceSurveyEmail {
    pub app_url: String,
    pub coach_name: String,
    pub competition_name: String,
    pub competition_url: String,
    pub round_name: String,
    pub date_start: String,
    /// `None` pour une journée à date fixe : la ligne « Clôture » disparaît et
    /// « Ouverture » devient « Se tient le ».
    pub date_end: Option<String>,
    pub deadline: String,
    pub equipes: Vec<EquipeLigneVm>,
}

/// La relance des silencieux. Mêmes champs que l'ouverture — seuls le titre,
/// l'accroche et le ton de l'encart d'échéance changent.
///
/// **Deux gabarits et non un paramétré** : c'est la convention des quatre
/// existants, et une condition sur « est-ce une relance ? » au milieu d'un HTML
/// d'e-mail se relit mal. Le prix est la duplication du bloc de style, que les
/// six gabarits paient déjà.
///
/// Sa `deadline` est celle de la campagne, éventuellement repoussée par une
/// réouverture : la relance ne porte pas d'échéance propre (R7).
#[derive(Template)]
#[template(path = "emails/fr_FR/competition_presence_reminder.html")]
pub struct PresenceReminderEmail {
    pub app_url: String,
    pub coach_name: String,
    pub competition_name: String,
    pub competition_url: String,
    pub round_name: String,
    pub date_start: String,
    pub date_end: Option<String>,
    pub deadline: String,
    pub equipes: Vec<EquipeLigneVm>,
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

    fn ligne(equipe: &str, n: u8) -> EquipeLigneVm {
        EquipeLigneVm {
            team_name: equipe.to_string(),
            yes_url: format!("https://kreek.example/presence/jeton{n}/oui"),
            no_url: format!("https://kreek.example/presence/jeton{n}/non"),
        }
    }

    fn sondage(date_end: Option<&str>, equipes: Vec<EquipeLigneVm>) -> String {
        PresenceSurveyEmail {
            app_url: "https://kreek.example".into(),
            coach_name: "Alice".into(),
            competition_name: "Ligue de Fer".into(),
            competition_url: "https://kreek.example/app/s/competitions/c/x".into(),
            round_name: "Journée 3".into(),
            date_start: "11/09/2026".into(),
            date_end: date_end.map(str::to_string),
            deadline: "09/09/2026".into(),
            equipes,
        }
        .render()
        .expect("le gabarit doit se rendre")
    }

    fn relance(equipes: Vec<EquipeLigneVm>) -> String {
        PresenceReminderEmail {
            app_url: "https://kreek.example".into(),
            coach_name: "Alice".into(),
            competition_name: "Ligue de Fer".into(),
            competition_url: "https://kreek.example/app/s/competitions/c/x".into(),
            round_name: "Journée 3".into(),
            date_start: "11/09/2026".into(),
            date_end: Some("18/09/2026".into()),
            deadline: "09/09/2026".into(),
            equipes,
        }
        .render()
        .expect("le gabarit doit se rendre")
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

    // ── Le sondage de présence et sa relance ─────────────────────────────────

    /// R1 — deux équipes, quatre boutons, quatre URL distinctes. C'est le cas
    /// que la maquette laissait en commentaire, et le seul où une boucle mal
    /// écrite se voit : avec une seule équipe, un `for` fautif rend la même
    /// chose qu'un rendu direct.
    #[test]
    fn deux_equipes_donnent_quatre_boutons_aux_quatre_url() {
        let h = sondage(
            Some("18/09/2026"),
            vec![
                ligne("Les Crocs du Chaos", 1),
                ligne("Les Choux de Bruxelles", 2),
            ],
        );

        for url in [
            "https://kreek.example/presence/jeton1/oui",
            "https://kreek.example/presence/jeton1/non",
            "https://kreek.example/presence/jeton2/oui",
            "https://kreek.example/presence/jeton2/non",
        ] {
            assert!(h.contains(url), "URL absente du rendu : {url}");
        }
        assert_eq!(
            h.matches("class=\"answer-btn").count(),
            4,
            "deux équipes doivent produire exactement quatre boutons"
        );
        assert!(h.contains("Les Crocs du Chaos") && h.contains("Les Choux de Bruxelles"));
    }

    /// L'intertitre porte le nom de l'équipe **aussi** pour un coach qui n'en a
    /// qu'une : c'est ce qui remplace la ligne « Ton équipe » de la maquette, et
    /// ce qui permet un seul chemin de rendu.
    #[test]
    fn une_equipe_donne_une_seule_paire_sous_son_nom() {
        let h = sondage(Some("18/09/2026"), vec![ligne("Les Crocs du Chaos", 1)]);

        assert_eq!(h.matches("class=\"answer-btn").count(), 2);
        assert!(h.contains("Les Crocs du Chaos"));
        assert!(!h.contains("jeton2"));
    }

    /// L'indication de bas de bloc est **hors de la boucle** : la répéter sous
    /// chaque paire ferait un e-mail bavard pour un coach à trois équipes, et
    /// c'est l'erreur qu'une accolade mal placée produit.
    #[test]
    fn l_indication_de_clic_ne_se_repete_pas_par_equipe() {
        let h = sondage(
            Some("18/09/2026"),
            vec![ligne("Une", 1), ligne("Deux", 2), ligne("Trois", 3)],
        );

        assert_eq!(
            h.matches("Un seul clic suffit").count(),
            1,
            "l'indication doit être rendue une fois, après la boucle"
        );
    }

    #[test]
    fn une_journee_a_date_fixe_n_annonce_pas_de_cloture() {
        let h = sondage(None, vec![ligne("Les Crocs du Chaos", 1)]);

        assert!(!h.contains("Clôture"), "une date fixe n'a pas de fin");
        assert!(h.contains("Se tient le"));
    }

    /// Les deux gabarits ont été écrits d'un même moule ; une confusion de
    /// fichier ne se verrait nulle part ailleurs — les champs sont identiques,
    /// donc tout se rend sans erreur.
    #[test]
    fn la_relance_ne_dit_pas_la_meme_chose_que_l_ouverture() {
        let ouverture = sondage(Some("18/09/2026"), vec![ligne("Les Crocs du Chaos", 1)]);
        let rappel = relance(vec![ligne("Les Crocs du Chaos", 1)]);

        assert!(ouverture.contains("Seras-tu là pour Journée 3 ?"));
        assert!(!ouverture.contains("Il te reste peu de temps"));

        assert!(rappel.contains("Il te reste peu de temps pour Journée 3"));
        assert!(rappel.contains("Tu n'as pas encore dit si tu serais là"));
        assert!(!rappel.contains("Seras-tu là pour"));
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

        // Les deux équipes ne sont pas décoratives : un gabarit qui rendrait la
        // première seulement passerait un contrôle à une équipe.
        let presence = PresenceSurveyEmail {
            app_url: "https://kreek.example".into(),
            coach_name: "VAL-coach".into(),
            competition_name: "VAL-competition".into(),
            competition_url: "https://kreek.example/VAL-url".into(),
            round_name: "VAL-journee".into(),
            date_start: "VAL-debut".into(),
            date_end: Some("VAL-fin".into()),
            deadline: "VAL-echeance".into(),
            equipes: vec![
                EquipeLigneVm {
                    team_name: "VAL-equipe-une".into(),
                    yes_url: "https://kreek.example/VAL-oui-une".into(),
                    no_url: "https://kreek.example/VAL-non-une".into(),
                },
                EquipeLigneVm {
                    team_name: "VAL-equipe-deux".into(),
                    yes_url: "https://kreek.example/VAL-oui-deux".into(),
                    no_url: "https://kreek.example/VAL-non-deux".into(),
                },
            ],
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
            "VAL-echeance",
            "VAL-equipe-une",
            "VAL-oui-une",
            "VAL-non-une",
            "VAL-equipe-deux",
            "VAL-oui-deux",
            "VAL-non-deux",
        ] {
            assert!(presence.contains(attendu), "présence : {attendu} absent");
        }

        let rappel = PresenceReminderEmail {
            app_url: "https://kreek.example".into(),
            coach_name: "VAL-coach".into(),
            competition_name: "VAL-competition".into(),
            competition_url: "https://kreek.example/VAL-url".into(),
            round_name: "VAL-journee".into(),
            date_start: "VAL-debut".into(),
            date_end: Some("VAL-fin".into()),
            deadline: "VAL-echeance".into(),
            equipes: vec![EquipeLigneVm {
                team_name: "VAL-equipe-une".into(),
                yes_url: "https://kreek.example/VAL-oui-une".into(),
                no_url: "https://kreek.example/VAL-non-une".into(),
            }],
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
            "VAL-echeance",
            "VAL-equipe-une",
            "VAL-oui-une",
            "VAL-non-une",
        ] {
            assert!(rappel.contains(attendu), "relance : {attendu} absent");
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
            sondage(
                Some("18/09/2026"),
                vec![ligne("Les Crocs du Chaos", 1), ligne("Les Choux", 2)],
            ),
            sondage(None, vec![ligne("Les Crocs du Chaos", 1)]),
            relance(vec![ligne("Les Crocs du Chaos", 1)]),
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
