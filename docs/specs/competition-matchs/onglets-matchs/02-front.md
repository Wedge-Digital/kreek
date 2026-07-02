# Onglets Résultats & Calendrier — Phase 2 : Architecture front

## Page hôte

`competition_detail.rs` — assemblage pur, zéro logique. Les deux onglets chargent leur contenu en lazy via `hx-get` au premier clic sur l'onglet.

La page hôte n'émet aucun événement DOM et ne contient aucune logique JS propre à ces onglets.

## Widgets

| Widget | BC | Endpoint GET | Trigger de chargement | Mode |
|---|---|---|---|---|
| Résultats tab | competitions | `/competitions/{id}/resultats?cursor={journee_id}` | clic onglet (initial) + sentinel (suite) | Lecture seule |
| Calendrier tab | competitions | `/competitions/{id}/calendrier?cursor={journee_id}` | clic onglet (initial) + sentinel (suite) | Lecture seule |

Aucun événement DOM entre ces deux widgets : ils sont indépendants.

## Pattern scroll infini (identique pour les deux onglets)

Le fragment retourné par le serveur contient les journées rendues **et** le sentinel suivant à la fin. Quand il n'y a plus de journée à charger, le serveur renvoie un fragment sans sentinel (scroll arrêté naturellement).

```html
<!-- Fragment retourné par GET /competitions/{id}/resultats?cursor={cursor} -->

<div class="section-block card-shadow">
  <!-- journée N -->
</div>
<div class="section-block card-shadow">
  <!-- journée N-1 -->
</div>

<!-- Sentinel — absent si dernière page -->
<div id="sentinel-resultats"
     hx-get="/competitions/{{id}}/resultats?cursor={{next_cursor}}"
     hx-trigger="intersect once"
     hx-target="#resultats-list"
     hx-swap="beforeend"
     hx-indicator="#sentinel-resultats">
  <div class="scroll-sentinel">Chargement…</div>
</div>
```

Le conteneur cible dans la page hôte :

```html
<!-- Onglet Résultats — dans competition_detail -->
<div id="resultats-list"
     hx-get="/competitions/{{id}}/resultats"
     hx-trigger="click from:#tab-resultats once"
     hx-swap="innerHTML">
  <div class="scroll-sentinel">Chargement…</div>
</div>
```

## Onglet Résultats

### Contenu par journée

Chaque journée est une section `.section-block` avec :
- Header : label journée + compteur de matchs
- Lignes de matchs dans l'ordre d'enregistrement

### États d'une ligne de match

| État | Affichage |
|---|---|
| **Terminé** | Score TD + sorties + date |
| **En cours de saisie** | Badge "En cours de saisie" (animé) + lien "Accéder au rapport" + date |

### Sens du scroll

Du plus récent (journée la plus haute) vers le plus ancien. Le cursor est l'identifiant de la dernière journée chargée ; le serveur renvoie les N journées précédentes.

### Logo / initiales

Chaque côté (domicile / visiteur) affiche :
- `<img>` si `home_logo` / `away_logo` est renseigné (URL de projection locale)
- `<div class="team-logo-initials">` avec les initiales sinon (2 premières lettres du nom d'équipe, calculées côté serveur)

## Onglet Calendrier

### Contenu par journée

Même structure `.section-block`, avec :
- Header : label journée + plage de dates + compteur de matchs
- Lignes compactes : logo 38px · nom · VS centré + date · nom · logo

### Sens du scroll

Du plus proche (prochaine journée non jouée) vers le plus lointain.

### Pas de score

Le calendrier n'affiche que le pairing (domicile vs visiteur), la date et les logos. Aucun score, aucun état de saisie.

## Logos — projection locale

Les logos des équipes sont stockés dans une projection locale du BC `competitions`, alimentée par les app events reçus à l'inscription d'une équipe à la compétition ou à la mise à jour de son logo. Le widget n'appelle aucun port inter-BC.

## Règles métier identifiées

- Un match sans rapport démarré reste dans Calendrier, quelle que soit sa date.
- Un match dont la date est future apparaît uniquement dans Calendrier (jamais dans Résultats).
- Un match terminé (rapport complété) apparaît uniquement dans Résultats avec son score.
- 3 journées chargées par page de scroll (initial + chaque chargement sentinel).
- L'onglet actif par défaut sur la page de détail reste **Classement**.
