# Phase 2 — Architecture front : l'encart du coach connecté

**Entrée** : `assets/rawpages/html/app-competition-detail-presence.html`, validée
en phase 1 (cinq états).

L'encart est un **second chemin de réponse**, pour quand l'e-mail se perd — dans
les indésirables, sous cinquante messages, ou parce que l'adresse du coach n'est
pas connue (R3). Il ne remplace pas le lien ; il rattrape.

## Ce que l'existant impose

### La page de détail est déjà une page d'assemblage

`competition-detail.html` compose six onglets par `hx-get`, et
`CompetitionDetailTemplate` ne porte que l'en-tête et le contenu de l'onglet
courant. L'encart s'y ajoute **sans que la struct gagne un champ** : un conteneur
`hx-get` au-dessus des onglets, chargé en `load`.

Le rendre par le handler de page aurait fait payer à chaque affichage — six
onglets, six requêtes — le chargement d'une campagne que la plupart des visites
ne concernent pas.

### Les données sont toutes dans `competitions`, sauf les noms d'équipe

| Besoin | Source |
|---|---|
| les campagnes ouvertes de la saison | `IPresenceSurveyRepository` (unité 1) |
| les équipes du coach dans cette saison | `ITeamInfoPort::find_enrolled_teams`, filtré sur `coach_id` |
| le nom d'une équipe | `ITeamInfoPort::find_team_names` |

**Aucun port à créer.** Le croisement se fait dans un service d'hydratation, comme
pour la page publique — `TeamInfoDto` n'atteint ni le handler ni le gabarit.

## Le widget

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `presence-call` | competitions | `GET …/{season_id}/presence-call` | `load` | — | lecture + mutation |

Une seule action, en `POST` sur le même préfixe :
`…/{season_id}/presence-call/answer`, corps `{ round_id, team_id, presence }`.

### Aucun événement DOM, et c'est un choix

L'action rend **directement le fragment de l'encart**, ciblé sur sa propre
racine en `outerHTML`. Pas de `HX-Trigger`, pas d'abonnement.

C'est l'inverse du choix de l'onglet d'administration, et pour une raison
précise : là-bas, deux widgets devaient réagir à toute mutation, donc l'événement
portait quelque chose. Ici il n'y a **qu'un consommateur — lui-même**. Émettre un
événement pour se prévenir soi-même serait de la cérémonie, et créerait un nom
global que la page hôte devrait ignorer.

Corollaire : la page de détail ne se recharge pas, et les onglets ne bougent pas.
Le coach répond sans perdre le classement qu'il était en train de lire.

## Les cinq états de la maquette n'en sont pas cinq

La maquette bascule entre « réponse attendue », « a répondu présent », « a répondu
absent », « deux équipes engagées » et « aucun sondage ouvert ». Vue du serveur,
la structure est plus simple :

```
rien du tout                                   ← aucune campagne ne concerne ce coach
une carte par campagne ouverte
    une ligne par équipe du coach dans la campagne
        selon sa réponse : deux boutons, ou un état et un bouton de bascule
```

« Deux équipes » est **le cas général**, dont les trois premiers états sont le
rendu à N=1. « Aucun sondage » ne rend rien — pas un encart vide, pas un message :
l'écran d'un coach qu'on ne sollicite pas doit être celui d'avant.

**Les deux mises en forme de la maquette sont conservées** : boutons en ligne à
une équipe, lignes empilées au-delà. La maquette a délibérément fait le cas
courant compact, et l'unifier en lignes aurait alourdi 95 % des affichages pour
la régularité du code.

## Une carte par campagne, pas une seule

R2 garantit une campagne vivante par **journée**, pas par compétition : deux
journées peuvent être sondées en même temps — un organisateur qui prépare deux
soirées d'affilée.

L'encart les affiche **toutes**, la plus proche échéance en premier. Cacher une
question parce qu'une autre existe ferait manquer une échéance, et l'encart n'a
qu'un rôle : dire qu'on attend quelque chose du coach.

Le cas normal reste une carte, et ça ne coûte qu'un `Vec` là où un `Option`
aurait suffi.

## Ce qui vit dans le navigateur

**Rien.** Pas d'Alpine, pas d'état local, pas de calque. Chaque bouton est un
`hx-post` qui rend le nouvel encart.

C'est la même décision qu'à l'onglet d'administration, où la maquette dessinait
un menu au « ⋯ » : un bouton par réponse coûte un aller-retour et ne demande
aucun état côté client.

## Règle métier apparue en phase 2

### R28 — Un coach connecté ne répond que pour ses propres équipes

L'encart n'offre que les équipes du coach — mais **l'offre n'est pas la garde**.
Le `POST` porte un `team_id` venu du navigateur, et R19 ne vérifie que
l'appartenance à la campagne, jamais à qui l'équipe appartient : un identifiant
forgé aurait posé une présence pour l'équipe d'un autre.

Le contrôle ne peut pas vivre dans le handler — ce serait une règle métier dans
la couche web. Il vit dans l'agrégat, qui savait déjà répondre : **`Reponse`
porte `coach_id`**.

```rust
pub enum Repondant { Jeton, Coach(CoachId), Organisateur(CoachId) }
```

Trois variantes pour trois chemins, parce qu'ils n'ont pas la même autorisation :
le jeton *est* l'autorisation (R7), la session ne l'est pas, et l'organisateur
passe par `require_admin_access`. `enregistrer` refuse un `Coach(id)` dont
l'identifiant ne correspond pas au `coach_id` de la réponse ; `Jeton` n'est pas
contrôlé, faute de quoi on comparerait la réponse à elle-même.

Détail complet dans `onglet-presences/06-domaine.md`, où l'agrégat vit.

**Ce que cette règle dit du découpage.** L'agrégat a été conçu d'un bloc pour les
trois unités, et il passe l'épreuve à moitié : la forme était bonne — les trois
appellent bien le même `enregistrer` — mais deux chemins avaient été fondus en
une variante, et seul le troisième appelant l'a fait voir. C'est l'argument du
workflow retourné : la troisième méthode révèle que les deux premières avaient la
mauvaise signature, sauf qu'ici c'est le troisième *appelant*.
