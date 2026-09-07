# Phase 4 — Contrats de données : l'encart du coach connecté

**Entrée** : `03-back.md` validé — un widget, une action, un service
d'hydratation, une requête de plus au port.

## Ce que l'existant impose

Rien de neuf : DTO plat puis smart constructor, VM avec `from_domain()`
co-localisé, DTO de port arrêté au service d'hydratation. Cette phase est
courte, et c'est attendu d'une troisième unité.

## Le DTO d'entrée — un seul

```rust
#[derive(Deserialize)]
pub struct PresenceCallAnswerBody {
    pub round_id:  String,
    pub team_id:   String,
    pub presence:  String,     // "presente" | "absente"
}
```

Identique à l'`AnswerBody` de l'onglet d'administration, et **ce n'est pas une
occasion de factoriser** : les deux vivent dans des BC layers différents, portent
des routes différentes, et se retrouveraient couplés au premier champ que l'un
gagnerait sans l'autre. Trois champs dupliqués coûtent moins qu'une dépendance
entre deux écrans.

Le handler construit `RecordAnswerCommand` avec **`Repondant::Coach(id du
connecté)`** — R28. L'identité vient de la session, jamais du corps.

## Ce que rend l'hydratation

```rust
pub struct CampagneOuverte {
    pub round_id:      MatchId,
    pub round_name:    String,
    pub round_dates:   String,
    pub deadline:      SurveyDeadline,
    pub confirmes:     usize,          // compte_presents() — du domaine
    pub mes_equipes:   Vec<MonEquipe>,
}

pub struct MonEquipe {
    pub team_id:   TeamId,
    pub team_name: String,
    pub presence:  Presence,           // du domaine, pas une chaîne
}
```

`presence` traverse en **type du domaine** : c'est le VM, en bout de chaîne, qui
choisit les mots. L'aplatir ici aurait mis la formulation dans la couche
applicative.

`confirmes` vient de **`compte_presents()`**, jamais d'un `filter().count()` sur
une liste. C'est exactement le défaut de la carte 495, où la vue recomptait ce
que le domaine savait compter, et comptait autre chose — ici elle ne verrait de
toute façon que les équipes du coach, et afficherait « 1 équipe a confirmé » là
où il y en a neuf.

## Les VM

```rust
#[derive(Template)]
#[template(path = "widgets/presence-call.html")]
pub struct PresenceCallTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub campagnes: Vec<CampagneVm>,     // vide ⇒ le gabarit ne rend rien
}

pub struct CampagneVm {
    pub round_id:    String,
    pub titre:       String,      // « Seras-tu là pour la journée 3 ? »
    pub sous_titre:  String,      // dates, échéance, nombre de confirmés
    pub deadline:    String,
    pub equipes:     Vec<EquipeLigneVm>,
}

pub struct EquipeLigneVm {
    pub team_id:   String,
    pub team_name: String,
    pub etat:      EtatEquipeVm,
}

pub enum EtatEquipeVm {
    Attendue,
    Presente { repondu_le: String },
    Absente  { repondu_le: String },
}
```

`EtatEquipeVm` est un enum et non trois booléens : le gabarit le `match`, et les
combinaisons absurdes — présente *et* absente, présente sans horodatage — sont
inexprimables. C'est la même raison qui a fait choisir `Presence` à données
portées dans le domaine, appliquée à la présentation.

### Le gabarit a le droit de demander `equipes.len()`

Le choix de mise en forme — boutons en ligne à une équipe, lignes empilées
au-delà — est une **question de vue**, et le test de la règle le dit : *si cette
valeur s'avérait fausse, corrigerait-on la vue ou le domaine ?* Ici la vue. Le
gabarit branche donc sur `{% if campagne.equipes.len() == 1 %}` sans qu'un champ
`compacte: bool` ait à voyager.

C'est la limite de « un view model transpose, il ne dérive pas » : elle interdit
de recalculer une **réponse métier**, pas de compter les éléments qu'on est en
train d'afficher.

## `campagnes` vide rend une page inchangée

Le gabarit ne produit **rien** — pas un encart vide, pas un message, pas une
bordure. L'écran d'un coach qu'on ne sollicite pas doit être celui d'avant.

Un `<div>` de zéro hauteur laisserait sa marge, et l'espacement de la page
bougerait selon qu'un sondage est ouvert ou non.

## Interfaces d'utilisation

| Type | Émis par | Consommé par |
|---|---|---|
| `PresenceCallAnswerBody` | le navigateur (htmx) | `presence_call_widget.rs` |
| `RecordAnswerCommand` (`Repondant::Coach(id)`) | `presence_call_widget.rs` | `record_answer_use_case` — celui de l'unité 1 |
| `PresenceSurvey` | `list_open_surveys_for_season` | le service d'hydratation |
| `TeamInfoDto` | `ITeamInfoPort` | **`presence_call_service` seul** |
| `CampagneOuverte`, `MonEquipe` | `presence_call_service` | les `from_domain()` |
| `CampagneVm`, `EquipeLigneVm`, `EtatEquipeVm` | `from_domain()` co-localisés | le gabarit, seul |

## Règle métier apparue en phase 4

### R29 — L'encart dit combien, jamais qui

Le sous-titre annonce « 9 équipes ont déjà confirmé ». Il n'annoncera jamais
*lesquelles*, ni qui a décliné.

**Ce n'est pas une omission d'affichage à corriger un jour.** Une réponse est
donnée à l'organisateur, qui apparie ; elle n'est pas publiée aux autres coachs.
Afficher la liste ferait de l'encart un tableau de présence collectif, avec deux
effets qu'aucune ligue ne demande : la pression sur celui qui n'a pas répondu, et
la possibilité de choisir sa soirée selon les adversaires présents — ce que le
tirage au sort existe précisément pour empêcher.

Le compte, lui, est utile et sans effet de bord : il dit que la soirée se remplit.

**Portée** : cette règle vaut pour l'encart et pour la page publique de réponse.
L'onglet d'administration montre évidemment les noms — c'est son objet.
