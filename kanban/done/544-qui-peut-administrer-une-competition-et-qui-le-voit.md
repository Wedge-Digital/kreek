# Qui peut administrer une compétition, et qui le voit

**Priorité : haute**
**Dépend de :** rien — la carte 416 a posé la garde serveur, qui est correcte
**Fichiers :** `src/app/competitions/use_cases/competition_admin_access_service.rs`
(nouveau), `src/app/competitions/io/web/competition_detail.rs`,
`calendrier_tab_controller.rs`, `resultats_tab_controller.rs`,
`admin/admin_page.rs`, `tests/e2e/`

## Objectif

Le bouton `⚙️ Administration` apparaît à qui a le droit d'entrer —
administrateur **de l'espace** ou administrateur **de la compétition** — et sur
**toutes** les entrées de la page, pas seulement sur l'onglet classement.

## Le symptôme

Un administrateur d'espace n'a pas le bouton sur la fiche d'une compétition
qu'il n'administre pas nommément. Il a pourtant le droit d'entrer : l'URL tapée
à la main fonctionne.

## Deux défauts, une seule cause — la règle est écrite deux fois

**Défaut 1 — la moitié de la règle.** `competition_detail.rs:466` ne consulte
que la liste des administrateurs de la compétition :

```rust
let is_admin = auth_session.user.as_ref().map_or(false, |user| {
    pb.admin_names.contains(&coach_name_str) || pb.admin_ids.contains(&user_id_str)
});
```

Le profil d'espace n'est jamais lu. Le mécanisme existe à trois lignes de là, et
quatre autres sites l'emploient correctement — `resultats_view.rs:97`,
`latest_results_view.rs:46`, `players/purchase_skill_controller.rs:115`,
`players/player_detail_controller.rs:127`. Un site sur cinq est fautif, et c'est
celui qui décide de l'affichage.

**Défaut 2 — six pages sur sept n'essaient même pas.** `full_page()` reçoit
`is_admin` en paramètre :

| Appelant | Valeur passée |
|---|---|
| `competition_detail.rs:466` (page racine) | calculée — à moitié |
| `competition_detail.rs:524, 564, 604, 644` | `false` |
| `calendrier_tab_controller.rs:282` | `false` |
| `resultats_tab_controller.rs:123` | `false` |

Ces six-là ne reçoivent pas `auth_session` : ils ne *peuvent* pas calculer le
droit, et le `false` est un remplissage. Arriver sur la compétition par un
onglet en chargement complet fait donc disparaître le bouton **y compris pour
l'administrateur de la compétition**. C'est le corollaire « un gabarit n'invente
aucune valeur » du CLAUDE.md, commis un cran plus haut, dans le contrôleur.

Les deux défauts expliquent ensemble un symptôme intermittent : le bouton
présent ou absent selon la porte par laquelle on est entré.

## Ce n'est pas un trou de sécurité

`require_admin_access` (`admin/admin_page.rs:97-127`) accorde bien l'accès à
`is_space_admin || is_comp_admin`, et vérifie en plus que la saison appartient à
la compétition. **Rien n'est ouvert à qui ne devrait pas.** On ne montre pas un
droit qu'on accorde — l'inverse du défaut habituel, et c'est pour ça qu'il a
survécu : un défaut trop restrictif ne déclenche aucune alarme, il agace.

## La décision — la règle à un seul endroit, et un type qui interdit l'oubli

**1. Le prédicat est extrait, et devient la seule source.** Nouveau
`use_cases/competition_admin_access_service.rs`, sur le modèle de
`roster_edit_access_service` (carte 500) :

```rust
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn peut_administrer(
    coach_id: &CoachId,
    coach_name: &str,
    space_id: &SpaceId,
    comp: &CompetitionBaseInfo,
    membres: &dyn ICompetitionSpaceMemberPort,
) -> bool
```

La compétition est interrogée **en premier** : ses `admin_ids` / `admin_names`
sont déjà en mémoire, portés par `CompetitionBaseInfo` et par `PageBase`.
L'espace ensuite, qui coûte un aller-retour. Un administrateur de compétition
consultant sa propre compétition, le cas fréquent, n'en déclenche aucun.

`require_admin_access` remplace son calcul par un appel à ce service. Une seule
règle, deux appelants — c'est ce qui empêche l'affichage et la garde de
redivenir.

**2. `full_page` calcule au lieu de recevoir.** Elle perd son paramètre
`is_admin: bool`, devient `async`, et prend `auth_session: &AuthSession` +
`state: &AppState`. Les sept appelants doivent alors fournir l'`AuthSession` —
que six n'ont pas, et qu'Axum injecte comme extracteur.

Le point n'est pas de corriger six `false`, c'est de rendre le septième
impossible : **le compilateur devient le verrou**, et un huitième onglet ajouté
demain naîtra correct. Ajouter `if is_space_admin` dans chaque appelant réglerait
le cas du jour et rien de plus — c'est exactement le raisonnement de la carte
500, et l'oubli qu'il évite.

Le gabarit ne change pas : `{% if is_admin %}` reste tel quel.

## Ce que la carte ne fait pas

- Elle ne touche à **aucune garde d'écriture** : `require_admin_access` garde la
  même règle, elle est seulement écrite ailleurs.
- Elle ne touche pas aux quatre autres sites qui calculent déjà correctement le
  droit, dans `competitions` et dans `players` — leur règle est bonne, et
  `players` ne peut de toute façon pas importer le service de `competitions`.
- Elle ne traite pas la carte 316 (audit d'appartenance à l'espace), qui vise le
  sens inverse : agir sur la ressource d'un autre espace.

## Point de vigilance

`full_page` frôle les 20 lignes (16 champs de template). L'appel au service doit
y tenir en une ligne, sinon la construction du template part dans une fonction
nommée.

## Checklist

- [x] `competition_admin_access_service::peut_administrer`, compétition d'abord
- [x] `require_admin_access` l'appelle — la règle n'est plus écrite deux fois
- [x] `full_page` async, sans paramètre `is_admin` ; les 7 appelants passent
      l'`AuthSession`
- [x] Tests unitaires du service : admin de compétition seul, admin d'espace
      seul, les deux, ni l'un ni l'autre
- [x] E2E : un admin d'espace **non** admin de la compétition voit le bouton
- [x] E2E : le même bouton est là en arrivant par l'onglet calendrier (défaut 2)
- [x] `make lint`, `make check-arch`, `make test`, `make e2e`

## Le test qui compte

Celui de l'admin d'espace **non** administrateur de la compétition, arrivant
**par l'onglet calendrier**. Il croise les deux défauts : corriger l'un sans
l'autre le laisse rouge. Un test sur la page racine avec un admin de compétition
passerait déjà aujourd'hui, et ne prouverait rien.

À voir échouer avant correction — sinon il ne prouve pas ce qu'on croit.


## Ce qui a été fait

**Les tests ont été vus échouer**, sur le code d'avant restauré depuis `HEAD` —
pas sur une neutralisation approchante. Les sept tests du bouton tombent, les
deux contre-épreuves restent vertes :

```
test_admin_espace_voit_le_bouton                       FAILED
test_membre_simple_ne_voit_pas_le_bouton               PASSED
test_le_membre_simple_reste_refuse_a_l_entree          PASSED
test_..._par_l_onglet[-, /calendrier, /resultats,
                      /detailed-standings, /teams, /stats]   FAILED (6)
```

Ce profil est ce qui compte : un correctif qui aurait ouvert le bouton à tout le
monde ferait tomber la deuxième ligne, et un qui aurait desserré la garde ferait
tomber la troisième.

Côté unitaire, neutraliser la moitié « espace » de la règle fait tomber
`admin_de_l_espace_seulement`, et lui seul — c'est bien ce défaut-là qu'il
éprouve, et non la forme du service.

### Deux écarts au plan

**La signature du service** prend une `AdminsDeLaCompetition` — les deux listes
empruntées — plutôt qu'un `&CompetitionBaseInfo`. `PageBase` porte les mêmes
listes sans porter ce type, et la struct nomme les deux paramètres là où deux
`&[String]` côte à côte laisseraient leur inversion passer.

**`require_admin_access` a été découpée.** Elle dépassait déjà les 20 lignes
avant cette carte ; la modifier sans la ramener sous la règle aurait figé la
dette. `charger_competition` en sort par copier-coller (règle 5), et la fonction
retombe à 20 lignes exactement.

### Résultats

`make test` 1741 passés · `make lint` vert · `make check-arch` 18 axes verts ·
`make e2e` 381 passés, 7 ignorés.
