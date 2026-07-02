# Competition Matchs — Spec index

Ajout des onglets **Résultats** et **Calendrier** à la page de détail d'une compétition.

## Contexte

La page `competition_detail` expose actuellement un onglet "Matchs" qui liste les matchs terminés, journée par journée, sans scroll infini ni gestion des états intermédiaires.

Cette fonctionnalité remplace cet onglet unique par deux onglets distincts :
- **Résultats** : matchs terminés + en cours de saisie, scroll infini du plus récent au plus ancien
- **Calendrier** : matchs à venir, scroll infini du plus proche au plus lointain

Les logos des équipes sont gérés via une **projection locale** dans le BC `competitions`.

## Progression

| Page / onglet | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| onglets-matchs | ✅ | ✅ | ✅ | — | ✅ | ✅ | ✅ |
