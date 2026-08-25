# Saisie des actions — gain de la Haine · Phase 5 : use cases

**Entrée** : `04-dtos.md` validé.

## Aucun use case nouveau

La mutation existe : `record_action_use_case::execute`. La Haine voyage **dans
l'action**, décision de la phase 4 — elle n'est pas un fait séparé, donc pas une
seconde mutation. Créer un `record_hatred_use_case` séparerait ce qui doit être
atomique : une blessure et sa Haine sont un seul événement, ou aucun.

## La signature gagne un port

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RecordActionCommand,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
    keywords: &dyn IKeywordCatalogPort,     // ← nouveau
) -> Result<RecordActionOutcome, RecordActionError>
```

`#[tracing::instrument(skip_all, fields(cmd = ?cmd))]` est déjà là et suffit :
`cmd` porte l'action, donc le mot-clef apparaît au journal sans qu'on ajoute un
champ. Rien de sensible ne s'y trouve — un uid de corpus n'est pas un secret.

## Orchestration

L'ordre importe : **on refuse avant de charger l'agrégat** quand c'est possible,
et on ne consulte le catalogue que si une Haine est déclarée.

```
1. Si l'action porte une Haine :
     a. la blessure la permet-elle ?        → sinon HatredNotAllowed
     b. l'uid existe-t-il au catalogue ?    → sinon UnknownKeyword
2. Charger le rapport                       → NotFound / NotInPreMatchPhase
3. Résoudre le joueur (nom, poste)          → PlayerNotFound / TempPlayerNotFound
4. pm.record_action(…)                      → l'agrégat produit ActionRecorded
5. repo.append(…)                           → Repository
```

**L'étape 1a n'est pas de la logique métier dans le use case.** La question
« cette blessure peut-elle donner la Haine ? » appartient au domaine : le use
case appelle une fonction de `InjuryType` définie en phase 6, il ne réimplémente
pas la liste des trois blessures. Ce qu'il fait ici, c'est **l'ordre** — refuser
tôt plutôt que tard.

**L'étape 1b est une consultation pure**, donc un port : « cet uid existe-t-il
dans le règlement, maintenant ? ». C'est le critère du `CLAUDE.md` entre port et
app event, et c'est aussi pourquoi le catalogue n'est pas projeté localement.

## Les erreurs

```rust
pub enum RecordActionError {
    NotFound,
    NotInPreMatchPhase,
    PlayerNotFound(String),
    TempPlayerNotFound(String),
    UnknownKeyword(String),      // ← nouveau — l'uid refusé
    HatredNotAllowed,            // ← nouveau — blessure qui n'en donne pas
    Domain(DomainError),
    Repository(String),
}
```

`UnknownKeyword` porte l'uid : sans lui, le journal dirait qu'un mot-clef a été
refusé sans dire lequel, et le premier corpus incomplet coûterait une
investigation. Les deux se traduisent en **422** au contrôleur, avec une ligne en
`warn` — le front rend ces cas impossibles à produire, donc leur apparition
signale un client hors de l'interface.

**`HatredNotAllowed` ne porte rien** : le type de blessure est déjà dans `cmd`,
donc dans le champ `cmd` du span. Le répéter dans l'erreur serait une deuxième
source de vérité pour la même information.

## Le troisième refus n'est pas ici

« `hate_gained = true` sans mot-clef » ne peut pas atteindre le use case : la
commande porte `Option<HatredKeyword>`, et l'état « oui sans lequel » n'est pas
représentable. **C'est le handler qui refuse**, en construisant la commande —
avant tout appel. Le typage a déplacé une règle de validation vers un endroit où
elle ne peut plus être oubliée.

## Ce que le use case ne fait pas

- **Il ne vérifie aucun doublon** — décision de la phase 3 : le panneau ne connaît
  pas les Haines déjà acquises, et s'en passer évite une consultation inter-BC.
- **Il n'écrit rien dans `players`.** La Haine part à la publication, par le
  mécanisme d'impact de match existant. Un use case qui écrirait tout de suite
  créerait le second chemin qu'on a refusé, et la dépublication laisserait la
  Haine derrière elle.
- **Il ne filtre pas le journalier.** La Haine d'un `ActionPlayer::Temp` est
  enregistrée dans l'action comme les autres ; c'est le **publisher** qui n'émet
  que pour les `Regular`, en phase 7.

## Le câblage

`record_action_controller` reçoit le port depuis `MatchReportContext`, qui le
reçoit de `main.rs`. Trois lignes, aucun état nouveau.

## Règles métier à préciser en phase 6

- **Quelles blessures donnent la Haine** : la liste des trois — Amoché, Blessure
  Sérieuse, Séquelle — devient une fonction du domaine, pas une constante du
  contrôleur ni une liste dupliquée dans le JS. La maquette la nomme déjà
  `PEUT_GAGNER_HAINE` ; le domaine doit en être la source.
- **Ce que l'agrégat vérifie encore** : `record_action` accepte-t-il une action
  déjà porteuse d'une Haine sans rien revalider, puisque le use case l'a fait, ou
  le domaine refait-il le contrôle ? La règle du projet veut que l'invariant vive
  dans l'agrégat ; reste à décider si le use case garde son refus précoce en plus.
