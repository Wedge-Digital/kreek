# Le nom du coach d'une équipe vient du navigateur

**Épic :** aucune · **Dépend de :** rien
**Priorité : moyenne** — un nom affiché que le serveur ne vérifie pas
**Trouvée par :** le plan de la carte 640 (2026-11-02).

## Le constat

À la création d'un brouillon d'équipe, le formulaire envoie `coach_id` **et**
`coach_name` (`src/app/team_creation/io/web/post_draft_team.rs:18-25`). Le
contrôleur reprend le nom tel quel, et vide s'il manque
(`form.coach_name.unwrap_or_default()`, `:104`), puis le passe à
`create_draft_team` (`:105-115`). Il suit `TeamCreated` jusqu'à
`team_proj.coach_name`.

Le serveur connaît pourtant le coach par son identifiant : le nom n'est qu'une
copie, et rien ne vérifie qu'elle correspond.

## Ce que ça produit

- **En e2e** : `build_and_submit_team_http` (`tests/e2e/competition_lifecycle.py`)
  poste `"coach_name": ""`. 5024 équipes sur 5259 de la base e2e
  (2026-11-02) ont un `coach_name` vide, et s'affichent sans coach dans les
  widgets d'inscription, de tirage et du calendrier. C'est aussi ce qui
  faisait sauter le test de la carte 640.
- **En production** : un client qui poste à la main peut inscrire un nom vide,
  ou celui d'un autre coach, à côté de son propre `coach_id`. Le nom ne sert
  qu'à l'affichage — les droits passent par `coach_id` (constat du plan de la
  640) — mais l'écran affirmerait alors qui coache l'équipe.

## Ce qu'il faut faire

1. Résoudre le nom **côté serveur** depuis `coach_id`, par le port qui sert
   déjà à `team_creation` pour connaître les coachs, et ignorer le champ
   `coach_name` du formulaire.
2. Décider du sort de `DraftTeamForm::coach_name` (le retirer, après avoir
   listé ses consommateurs — règle 4).
3. Mesurer en production combien d'équipes ont un `coach_name` vide ou
   différent de celui de leur `coach_id`, avant de décider d'une reprise.

## Hors du périmètre

La reprise des équipes existantes : elle se décide sur la mesure du point 3,
et c'est alors une migration de données à part.

## Terminé quand

- [ ] un brouillon créé avec un `coach_name` faux ou vide porte le nom du
      compte de son `coach_id` ;
- [ ] les équipes créées par les e2e s'affichent avec leur coach.

## Conséquences sur les données de production

Aucune écriture rétroactive : seules les équipes créées après la livraison
portent le nom résolu. Les existantes restent telles quelles jusqu'à une
éventuelle reprise.
