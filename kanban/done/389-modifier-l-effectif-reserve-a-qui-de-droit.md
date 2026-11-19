# « Modifier l'effectif » réservé à qui de droit

**Priorité : haute**
**Dépend de :** rien
**Fichiers :** `src/app/teams/io/web/team_detail.rs`,
`src/app/teams/ports.rs`, `src/app/teams/use_cases/roster_edit_access_service.rs`,
`src/app/teams/context.rs`, `src/infrastructure/teams/access_adapter.rs`,
`src/main.rs`

## Objectif

Sur la fiche d'équipe (`/app/{space_id}/teams/{team_id}`), le bouton
`✎ Modifier l'effectif` n'apparaît que pour :

- le **coach propriétaire** de l'équipe,
- un **administrateur de l'espace**,
- un **administrateur de la compétition** où l'équipe est inscrite.

Pour tout autre visiteur, le bandeau reste affiché avec son texte et son bouton
`Imprimer en PDF` — seul le déclencheur d'édition disparaît.

## L'état actuel : un bouton menteur, pas une faille

`BannerCtaVm::RosterEdit` est ajouté dès que l'équipe est `Enrolled` +
`ReadyToPlay`, sans aucune condition. Le handler `team_detail` ne prend même
pas `AuthSession` : **il ne sait pas qui regarde**.

L'écriture, elle, est déjà gardée. `post_update_roster` refuse par
`can_spend_spp`, qui est exactement la règle ci-dessus. Un visiteur qui clique
entre en édition, saisit, enregistre et reçoit un **403** — il a perdu sa
saisie pour découvrir un droit qu'on aurait pu lui dire au premier coup d'œil.

C'est ce qu'on corrige : l'affichage rejoint l'autorisation. Aucune écriture
n'est ouverte ni fermée par cette carte.

## La règle est dans `players`, le bouton dans `teams`

`can_spend_spp` vit dans `players/io/web/purchase_skill_controller.rs` et
s'appuie sur les ports de `players`. `teams` ne peut pas l'appeler : les deux
BCs ne s'importent pas.

`teams` a déjà la moitié de la réponse dans son agrégat — `Team` porte
`coach_id`, `space_id` et `competition_id`. **La propriété se décide sans
aucun port.** Ne manquent que les deux questions d'administration, qui vivent
dans `spaces` et `competitions`.

### Un port, pas deux

```rust
#[async_trait]
pub trait ITeamAccessPort: Send + Sync {
    async fn is_space_admin(&self, coach_id: &CoachId, space_id: &SpaceId) -> bool;
    async fn is_competition_admin(
        &self,
        competition_id: &CompetitionId,
        coach_id: &CoachId,
        coach_name: &str,
    ) -> bool;
}
```

`players` a deux ports séparés (`IPlayerSpaceMemberPort`,
`IPlayerCompetitionPort`) parce qu'il s'en sert aussi ailleurs. Ici les deux
méthodes ne servent qu'à une seule question — « ce visiteur a-t-il un droit
qu'il ne tient pas de la propriété ? » — et un port unique évite deux
câblages dans `main.rs` pour un seul appelant.

L'adapter, `src/infrastructure/teams/access_adapter.rs`, prend
`ISpaceRepository` et `ICompetitionRepository` : mêmes dépôts que les deux
adapters de `players`, dont il reprend le corps — une trentaine de lignes
chacun.

**`coach_name` en plus de `coach_id`** parce qu'une compétition stocke ses
administrateurs des deux façons (`admin_ids` **et** `admin_names`), et que
`can_spend_spp` interroge les deux. Reprendre l'un sans l'autre priverait du
bouton des administrateurs qui l'ont aujourd'hui.

## La décision

```rust
// arch:no-instrument — service de lecture : répond à une question de droit,
// n'exécute aucune intention métier
pub async fn peut_modifier_effectif(
    team: &Team,
    viewer_id: &CoachId,
    viewer_name: &str,
    access: &dyn ITeamAccessPort,
) -> bool
```

Propriété d'abord — c'est la seule des trois qui ne coûte aucun aller-retour.
Puis admin d'espace, puis admin de compétition, chacune court-circuitant les
suivantes.

`BannerVm::from_domain` reçoit le booléen et n'ajoute `RosterEdit` que s'il est
vrai. Le template ne change pas : il rend les CTA que le VM lui donne, et le
script du bandeau sort déjà proprement si le déclencheur est absent
(`if (!declencher) return;`).

## Une règle écrite trois fois

Après cette carte, « propriétaire ou admin d'espace ou admin de compétition »
existera dans `can_spend_spp`, dans `peut_modifier_effectif`, et dans une
troisième variante — `can_customise`, qui exclut délibérément le propriétaire.

C'est le prix de la souveraineté des BCs, et il est assumé. Ce qui doit être
tenu, c'est que les trois **portent des noms qui disent ce qu'elles
autorisent** : `check_admin_rights` a déjà été renommé pour cette raison. Une
quatrième copie, en revanche, mériterait qu'on s'arrête.

## Ce que la carte ne fait pas

- Elle n'ouvre ni ne ferme aucune écriture : `post_update_roster` garde
  `can_spend_spp` à l'identique.
- Elle ne touche pas au bouton `✏️ Customiser` de la fiche joueur, réservé aux
  administrateurs, dont l'exclusion du coach est un choix de conception.
- Elle ne touche pas à la maquette morte `/app/{space_id}/team/{team_id}/detail`
  (`team_creation`), qui sert des données en dur et quatre boutons sans action,
  dont un « Customiser ». Elle est servie en production et mérite sa propre
  carte.

## Checklist

- [x] `ITeamAccessPort` dans `teams/ports.rs`
- [x] `access_adapter.rs`, câblé dans `main.rs`, porté par `TeamsContext`
- [x] `roster_edit_access_service.rs` + marqueur `arch:no-instrument`
- [x] `team_detail` prend `AuthSession` ; sans session → pas de CTA
- [x] `BannerVm::from_domain` conditionne `RosterEdit`, laisse `Print`
- [x] Six tests unitaires sur port factice, plus un septième sur le bandeau
- [x] Test e2e, **vu échouer** — `Locator expected to have count '0', actual: 1`
- [x] `make lint`, `make check-arch`, `make test` — 1266 tests

## Ce qui a été fait

Le port compte ses appels dans les tests, et c'est ce qui permet de vérifier
l'ordre des trois questions : un propriétaire qui regarde son équipe — le cas
de loin le plus fréquent — ne déclenche **aucun** aller-retour, et une équipe
hors compétition n'interroge pas le port des compétitions. Sans ces
assertions-là, l'ordre serait une intention non tenue.

L'adapter rend `false` quand un dépôt échoue : le bouton disparaît plutôt que
d'apparaître à tort. L'écriture restant gardée par `can_spend_spp`, le pire cas
est un administrateur privé de son raccourci, jamais un visiteur qui gagne un
droit.

### Un septième test que la carte ne demandait pas

Celui du bandeau. La carte listait « coach tiers → pas de bouton, bandeau et
`Imprimer` toujours là », mais rien ne tenait la seconde moitié. Un correctif
qui aurait vidé les CTA — ou masqué le bandeau entier — aurait passé le test du
bouton absent. Il compare désormais les deux rendus et vérifie que seul le
déclencheur d'édition change.

### La question laissée ouverte est réglée

« Le jeu de données e2e porte-t-il un second coach dans le même espace ? » —
oui : `X-Bypass-Auth-Profile: simple` connecte « E2E Coach 01 », membre simple,
et trois fichiers s'en servent déjà. Le test e2e ouvre un **contexte de
navigateur à part** : l'en-tête se pose à sa création, et le partager
connecterait tous les autres tests en membre simple.

Il porte sa contre-épreuve : sans elle, il passerait aussi bien si le bouton
avait disparu pour tout le monde.

### Le piège de l'axe 11

`arch:no-instrument` n'est reconnu que sur **la seule ligne qui précède** la
fonction. Écrit sur deux lignes, il échoue — et le message ne dit pas pourquoi,
il se contente de nommer la fonction non instrumentée. Le motif est désormais
sur une ligne, avec un commentaire qui prévient le prochain.
