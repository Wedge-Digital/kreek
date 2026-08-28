# Compétences personnalisées

**Maquette :** `assets/rawpages/html/app-custom-skills.html`

## La fonction

Créer des compétences propres à un espace, qui s'ajoutent à celles du règlement
pour ses rosters et ses joueurs.

## Ce qui rend cette fonctionnalité légère

**Aucune compétence n'a d'effet mécanique dans kreek.** Le code ne lit que trois
choses :

| Ce que le code lit | Où |
|---|---|
| `is_elite` → le coût en SPP et la hausse de valeur | `improvement_cost_service.rs:36` et `:61` |
| la catégorie → l'accès primaire ou secondaire d'un poste | partout où l'on choisit une compétence |
| le nom et la description → l'affichage | les écrans |

Le seul `"DODGE"` codé en dur du projet est dans un double de test
(`roster_service.rs`, sous `#[cfg(test)]`). **Personne ne simule « Blocage »** —
le coach applique la règle sur sa table, l'application tient la liste et le
compte.

Une compétence personnalisée fonctionne donc **de bout en bout dès qu'elle est
au catalogue**, contrairement à un roster qui devait entrer dans un tier, être
choisi à la création d'équipe et produire des postes.

## Règles tranchées en phase 1

1. Une compétence personnalisée est **propre à son espace**.
2. Elle peut appartenir à **n'importe quelle catégorie**, `TRAITS` comprise.
3. Son **type** — `Standard` ou `Élite` — est saisi à la création.
4. Son `uid` porte le préfixe **`CUSTOM_`**, engendré côté serveur.
5. **Une compétence employée n'est pas supprimable.**
6. **Son libellé reste modifiable** — nom et description. Sa catégorie et son
   type sont **figés** : ils décident du coût, et des joueurs l'ont déjà payé.

## Les pages

| Page | État |
|---|---|
| `page-de-gestion/` | **workflow complet** — cartes 463 à 472 |
