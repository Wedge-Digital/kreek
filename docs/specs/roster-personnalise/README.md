# Roster personnalisé, propre à un espace

**Épic :** E10 — Référentiels éditables · **Carte d'origine :** 50
**Maquette :** `assets/rawpages/html/app-roster-editor.html`

## La fonction

Créer de toutes pièces un roster — identité, règles, postes — qui n'est
disponible que dans l'espace où il a été créé.

Aujourd'hui les rosters sont figés dans le corpus de références, lu au
démarrage depuis `REFERENCES__DIR`. En ajouter un demande de toucher un
fichier hors du dépôt **et de redémarrer le serveur**.

## Les pages

| Page | État |
|---|---|
| `editeur-de-roster/` | phases 1 et 2 faites |

## Règles tranchées en phase 1

- Les rosters personnalisés **vivent en base**, pas dans le corpus.
- Leur `uid` porte un **préfixe convenu** (`CUSTOM_`), qui dit où aller chercher.
- La création peut attribuer **n'importe quelle Haine**.
- **Un roster utilisé ne peut être ni modifié ni supprimé.**
