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

- [ ] `ITeamAccessPort` dans `teams/ports.rs`
- [ ] `access_adapter.rs` dans `src/infrastructure/teams/`, câblé dans
      `main.rs` et porté par `TeamsContext`
- [ ] `roster_edit_access_service.rs` + marqueur `arch:no-instrument`
- [ ] `team_detail` prend `AuthSession` ; visiteur sans session → pas de CTA
- [ ] `BannerVm::from_domain` conditionne `RosterEdit`, laisse `Print`
- [ ] Tests unitaires, sur un port factice :
  - [ ] coach propriétaire → bouton
  - [ ] admin d'espace non propriétaire → bouton
  - [ ] admin de compétition par id → bouton ; par nom → bouton
  - [ ] coach tiers → **pas** de bouton, bandeau et `Imprimer` toujours là
  - [ ] équipe sans compétition et visiteur tiers → pas de bouton, aucun appel
        au port compétition
- [ ] Test e2e : un coach tiers ouvre la fiche d'une équipe prête à jouer et
      n'y voit pas le bouton — **à vérifier avant d'écrire** : le jeu de
      données e2e porte-t-il un second coach dans le même espace ?
- [ ] `make lint`, `make check-arch`, `make test`, tests e2e impactés
