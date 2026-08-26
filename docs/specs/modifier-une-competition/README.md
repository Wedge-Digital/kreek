# Modifier une compétition en cours — Progression

Ajuster les paramètres d'une compétition **après son démarrage**, depuis
l'administration. Aujourd'hui tout se fige à la création : une faute dans le nom,
un barème mal choisi ou une poule de trop ne se corrigent pas.

## Maquette (Phase 1 ✅)

`assets/rawpages/html/app-competition-admin-modification.html` — validée et
commitée (`8957f2d`).

## Progression

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| Onglet Paramètres | ✅ | ✅ | | | | | |

## Ce qui est modifiable, et ce qui ne l'est pas

| Modifiable | Verrouillé |
|---|---|
| Nom, logo, nom de la saison | Rosters autorisés par tier |
| Points de victoire, nul, défaite | Budget de création par tier |
| Bonus offensif, défensif, agressif | XP de départ par tier |
| Critères de départage et leur ordre | Ajout ou retrait d'un tier |
| Visibilité — libre ou sur invitation | |
| Noms des poules, ajout et retrait | |
| Coups de pouce autorisés par tier | |

**Le verrou a une seule raison** : ces réglages ont servi à valider les équipes
déjà créées. Les changer obligerait à toutes les revalider.

## Décisions prises en phase 1

**Deux onglets disparaissent** de l'administration : « Tableau de bord » et
« Résultats ». Leurs routes — `admin_dashboard`, `admin_results` — sont
**retirées**, pas seulement déliées de la barre.

**Le Résumé devient l'onglet par défaut.**

**La conséquence d'un réglage est un état du formulaire**, jamais une alerte
posée dessus ni une modale. Elle est visible au repos, en gris, et prend un ton
d'accent avec un décompte dès qu'une modification existe — un administrateur doit
savoir ce qu'il engage **avant** de toucher au réglage.

**Une poule retirée reste à l'écran**, barrée, avec « 6 équipes à réaffecter », et
le geste s'annule. On voit ce qu'on défait.

**Deux composants sont repris de la page de création** : le sélecteur de coups de
pouce (`inducement-picker` de `references`) et les blocs de tier. Même geste,
même rendu — et une seule UX à maintenir.

## Le point dur : le recalcul du classement

Modifier le barème oblige à recalculer le classement de toutes les équipes.
**C'est faisable**, et l'instruction l'a vérifié :

- `ranking_lines` porte **une ligne par équipe et par match**, avec un `sequence`,
  le `match_report_id`, et les **statistiques brutes** — `td_for`, `td_against`,
  `casualties`, `fouls`, `completions` — à côté des points calculés ;
- `RankingLine::record_match(previous, ctx, stats, rules)` est une **fonction
  pure qui prend les règles en paramètre** ;
- `revert_match_ranking_use_case` prouve que le retrait d'un match est déjà
  maîtrisé.

Recalculer, c'est repartir de zéro et rejouer les matchs dans l'ordre de
`sequence` avec le nouveau barème.

**Un classement publié change directement** — sans annonce ni gel. L'écran
prévient avant l'enregistrement ; c'est le seul avertissement, et il suffit.

**Le recalcul appartient à `ranking`, pas à `competitions`.** Les cinq panneaux
écrivent tous dans `competitions`, barème compris. Mais `ranking_lines` est à
`ranking`, qui consulte déjà les règles par un port sans que `competitions` ne
connaisse le classement. Le lien est donc un **app event** : l'un publie que le
barème a changé, l'autre recalcule.

**Retirer une poule ne touche aucun résultat.** `ranking_lines` ne porte **aucune
colonne de poule** : le classement est tenu par saison, et le groupement n'est
qu'un filtre d'affichage. Les matchs ne sont pas limités aux membres d'une poule
et restent joués quelle que soit la répartition.

## Hors périmètre

- **Le panneau Administrateurs** de la maquette : il vient de l'ancienne maquette
  de paramètres et ne fait pas partie de la demande.
- **La phase finale (play-offs)**, retirée par la carte 412.
