# Supprimer `src/_bckp/`

**Priorité : moyenne**
**Répertoire :** `src/_bckp/`

## Problème

Un répertoire entier de code mort est committé dans le repo. Il contient d'anciennes implémentations du bus de commandes et de l'infrastructure (209 lignes de code Rust).

Conséquences :
- Pollue les résultats de recherche (`grep`, `find`)
- Fausse les métriques de couverture de tests
- Crée de la confusion sur ce qui est actif vs archivé

## Action

```bash
git rm -r src/_bckp/
```

L'historique git conserve le code si besoin de le retrouver. Un répertoire `_bckp/` committé n'est pas une stratégie d'archivage.
