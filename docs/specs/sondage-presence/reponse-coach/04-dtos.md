# Phase 4 — Contrats de données : la réponse du coach

**Entrée** : `03-back.md` validé — le lien à un jeton, le routeur public, le
gabarit propre au BC, deux `NotificationType`, `find_by_token`.

## Ce que l'existant impose

**`find_team_names(&[String])` existe déjà** sur `ITeamInfoPort`, à côté de
`find_enrolled_teams`. C'est exactement ce dont la page et l'e-mail ont besoin —
nommer une équipe, ou les N équipes d'un coach. **Aucune méthode de port à
ajouter.**

**DTO plat, puis smart constructor.** Le chemin arrive en `String` ; la
conversion se fait dans le handler, et son échec est un `404`, jamais un refus
métier.

**Pas de `builders.rs`.** La règle du `CLAUDE.md` le réserve aux VM qui dépendent
de DTO de port ; ici un service d'hydratation rend un objet local, donc les VM
ont leur `from_domain()` co-localisé.

## Le DTO d'entrée — un seul

```rust
#[derive(Deserialize)]
pub struct PresenceLinkPath {
    pub token: String,
    pub venue: String,     // "oui" | "non"
}
```

`SurveyToken::try_new(token)` et `Venue::try_from_segment(venue)` dans le
handler. Un troisième mot rend `404` — le chemin n'existe pas, il n'y a rien à
expliquer à qui l'a fabriqué.

**Pas de corps, pas de query.** Tout ce que la route reçoit tient dans son
chemin, et c'est ce qui la rend cliquable depuis un client mail.

## Ce que la page doit dire, et que l'agrégat ne porte pas

La maquette affiche « Journée 3 · du 12 au 19 octobre », « Ton équipe : Les Rats
d'Égouts », et un lien vers la compétition. **`PresenceSurvey` ne connaît aucun
de ces libellés** : il porte des identifiants, une échéance et des réponses.

C'est le trou que cette phase trouve, et que la phase 3 n'avait pas nommé. Il se
comble par un **service d'hydratation**, pas par des ports appelés depuis le
handler :

```rust
// use_cases/presences/presence_landing_service.rs

// arch:no-instrument — service d'hydratation : assemble une vue, sans intention métier
pub async fn hydrater(
    survey: &PresenceSurvey,
    token: &SurveyToken,
    labels_repo: &dyn IPresenceSurveyRepository,
    team_port: &dyn ITeamInfoPort,
) -> Option<LandingContext>
```

```rust
pub struct LandingContext {
    pub team_name: String,
    pub round_label: String,
    pub competition_name: String,
    pub competition_url: String,
    pub deadline: String,
    pub presence: Presence,        // du domaine, pas une chaîne
    pub statut: SurveyStatus,      // du domaine
}
```

**`TeamInfoDto` s'arrête ici.** C'est la règle « Domain services pour données
inter-BCs » : ni le handler ni le gabarit ne voient un DTO de port. Et
`presence` comme `statut` traversent en **types du domaine** — c'est le VM, en
bout de chaîne, qui choisit les mots.

Les libellés de journée, de compétition et de saison viennent d'une requête de
lecture du BC lui-même :

```rust
pub struct LandingLabelsDto {   // DTO de lecture — primitives assumées
    pub round_name: String,
    pub round_dates: String,
    pub competition_id: String,
    pub competition_name: String,
    pub season_id: String,
    pub space_id: String,
}
```

Elle vit dans `sql/presences/find_landing_labels.sql` et joint les tables du BC
`competitions` seul — jamais celles de `teams`, dont la souveraineté impose le
port.

## Les trois structs de template, pas quatre

| Struct | Gabarit | États couverts |
|---|---|---|
| `PresenceAnswerTemplate` | `public/presence-response.html` | présence confirmée **et** absence enregistrée |
| `PresenceClosedTemplate` | `public/presence-closed.html` | sondage clos |
| `PresenceUnknownTemplate` | `public/presence-unknown.html` | lien inconnu |

**Les deux premiers états partagent leur gabarit** parce qu'ils ne diffèrent que
par leurs mots : le même encadré, le même récapitulatif, le même bouton opposé
en dessous. Les séparer aurait dupliqué un markup qui doit rester en phase — et
c'est le contraire du choix fait pour les six panneaux de l'onglet, dont les
structures diffèrent réellement.

```rust
pub struct PresenceAnswerTemplate {
    pub app_url: String,
    pub recap: RecapVm,
    pub reponse: ReponseVm,
}

pub struct ReponseVm {
    pub venue_presente: bool,      // choisit la pastille et le titre
    pub titre: String,
    pub message: String,
    pub bouton_oppose_url: String, // R4 — même jeton, l'autre verbe
    pub bouton_oppose_label: String,
}

pub struct RecapVm {
    pub round_label: String,       // « Journée 3 · du 12 au 19 octobre »
    pub team_name: String,
    pub competition_name: String,
    pub competition_url: String,
}
```

`venue_presente` est un booléen **de présentation**, projeté depuis
`Presence::Declaree { venue, .. }` par `from_domain()`. Le gabarit choisit une
pastille ; il ne décide pas laquelle, et il ne relit aucune donnée pour la
déduire.

### L'état clos porte la dernière réponse reçue

```rust
pub struct PresenceClosedTemplate {
    pub app_url: String,
    pub recap: RecapVm,
    pub deadline: String,
    pub derniere_reponse: String,   // « présent », « absent », ou « aucune »
    pub motif: MotifFermeture,      // corrigé en phase 5 — R27
}

pub enum MotifFermeture { Echeance, Decision, JourneeJouee }
```

La maquette l'affiche déjà — « Dernière réponse reçue : aucune ». Sans ce champ,
le coach qui arrive après l'échéance ne saurait pas ce qui est enregistré à son
nom, et c'est précisément la question qu'il se pose en cliquant.

Le mot « clos » et non « expiré » : dire « lien expiré » laisserait croire à une
panne, alors que c'est la date qui a tranché.

**`motif` a été ajouté en phase 5, par R27.** Trois situations distinctes ferment
le chemin du coach — l'échéance, la décision de l'organisateur, et la journée
déjà jouée — et cette phase les faisait toutes tomber sur les mêmes mots. La
troisième n'est pourtant pas une clôture : la campagne peut être ouverte et son
échéance à venir, ce que l'écran affiche juste en dessous.

### L'état inconnu ne porte rien du tout

```rust
pub struct PresenceUnknownTemplate {
    pub app_url: String,
}
```

**Un seul champ, et c'est une décision, pas une économie.** Cf. R26 ci-dessous.

## Les deux structs d'e-mail

```rust
#[derive(Template)]
#[template(path = "emails/fr_FR/competition_presence_survey.html")]
pub struct PresenceSurveyEmail {
    pub app_url: String,
    pub coach_name: String,
    pub competition_name: String,
    pub competition_url: String,
    pub round_name: String,
    pub round_dates: String,
    pub deadline: String,
    pub equipes: Vec<EquipeLigneVm>,    // R1
}

pub struct EquipeLigneVm {
    pub team_name: String,
    pub yes_url: String,
    pub no_url: String,
}
```

`PresenceReminderEmail` porte les mêmes champs, plus rien : c'est le même
contenu sous un autre titre, et la relance ne dit pas autre chose que « tu n'as
pas encore répondu ».

**`equipes` est un `Vec` même pour un coach à une seule équipe.** R1 porte sur
l'équipe ; un champ scalaire aurait obligé à un second gabarit le jour où
quelqu'un en engage deux — c'est-à-dire dès la première ligue un peu vivante.

**Les URL sont construites par le mailer**, pas par le gabarit : elles composent
`app_url`, le chemin public et le jeton, et un gabarit qui les assemblerait
mettrait la forme du lien dans du HTML d'e-mail, hors de portée de tout test.

## Interfaces d'utilisation

| Type | Émis par | Consommé par |
|---|---|---|
| `PresenceLinkPath` | le navigateur, depuis l'e-mail | `presence_response.rs` |
| `RecordAnswerCommand` (`Repondant::Coach`) | `presence_response.rs` | `record_answer_use_case` — celui de l'unité 1 |
| `PresenceSurvey` | `find_by_token` | le handler, puis `presence_landing_service` |
| `LandingLabelsDto` | `IPresenceSurveyRepository` | **`presence_landing_service` seul** |
| `TeamInfoDto` | `ITeamInfoPort::find_team_names` | **`presence_landing_service` seul** — jamais un handler ni un gabarit |
| `LandingContext` | `presence_landing_service` | `presence_response.rs`, puis les `from_domain()` |
| `RecapVm`, `ReponseVm` | `from_domain()` co-localisés | les gabarits Askama, seuls |
| `PresenceSurveyEmail`, `PresenceReminderEmail`, `EquipeLigneVm` | `survey_mailer.rs` | les gabarits d'e-mail, seuls |

La ligne qui compte, la même qu'en unité 1 : **les DTO de port s'arrêtent au
service d'hydratation.**

## Règle métier apparue en phase 4

### R26 — La page publique ne dit pas si un jeton a existé

Un jeton inconnu, un jeton révoqué et un jeton tronqué par un client mail
rendent **la même page**, sans un mot de plus. La maquette le notait déjà en
commentaire ; c'est ici que cela devient une contrainte de type — le VM de cet
état ne porte aucun champ, donc il n'a rien à divulguer même par mégarde.

**Pourquoi cela compte** : la route est publique et le jeton est un `SUlid`
énumérable en principe. Une page qui distinguerait « ce lien n'a jamais existé »
de « ce lien ne répond plus » ferait de la page un oracle — on saurait, en
essayant, quels jetons sont vivants.

Le coût est réel et assumé : un coach dont le lien a été tronqué par sa
messagerie ne saura pas que c'est la cause. La page le lui suggère — *« il est
peut-être incomplet, certains logiciels de messagerie coupent les liens
longs »* — sans rien confirmer.
