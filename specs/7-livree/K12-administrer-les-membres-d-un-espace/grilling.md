# K12 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de l'épic `kanban/epics/done/E12-administrer-les-membres-d-un-espace.md` (devenue `initiative.md`). Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K12-B1** — Terminé quand (critère de l'épic E12) : Sur la démo, un administrateur d'espace ouvre `/app/{space_id}/admin`, y voit ses membres avec leur rôle, **promeut puis rétrograde** un coach, **en retire** un, et **fait entrer deux nouveaux venus** — l'un depuis l'annuaire, l'autre par création de compte. Le second **reçoit son email de définition de mot de passe et parvient à se connecter**. Un `SpaceUser` qui ouvre la même URL reçoit un 403, et la page ne produit **aucun décalage** au chargement. Le critère enchaîne délibérément la création de compte jusqu'à la connexion : c'est le seul bout par lequel on vérifie que le contrat `accountCreated` tient et que l'email part vraiment.
- **K12-B2** — Deux événements de `spaces` portent le même type (ticket `364-deux-evenements-de-spaces-portent-le-meme-type`)
- **K12-B3** — L'appartenance à un espace devient un invariant (ticket `365-l-appartenance-a-un-espace-devient-un-invariant`)
- **K12-B4** — Le dépôt sait lire, modifier et retirer une appartenance (ticket `366-le-depot-sait-lire-modifier-et-retirer-une-appartenance`)
- **K12-B5** — Changer le rôle et retirer un membre, côté applicatif (ticket `367-changer-le-role-et-retirer-un-membre-cote-applicatif`)
- **K12-B6** — La page d'administration et sa barre d'onglets (ticket `368-la-page-d-administration-et-sa-barre-d-onglets`)
- **K12-B7** — La liste des membres (ticket `369-la-liste-des-membres`)
- **K12-B8** — Les statistiques de l'espace (ticket `370-les-statistiques-de-l-espace`)
- **K12-B9** — Changer un rôle, retirer un membre, depuis l'écran (ticket `371-changer-un-role-retirer-un-membre-depuis-l-ecran`)
- **K12-B10** — Le bouton de réinitialisation de mot de passe (ticket `372-le-bouton-de-reinitialisation-de-mot-de-passe`)
- **K12-B11** — `competitions` réagit au retrait d'un membre (ticket `373-competitions-reagit-au-retrait-d-un-membre`)
- **K12-B12** — La page d'administration sous Playwright (ticket `374-la-page-d-administration-sous-playwright`)
- **K12-B13** — L'agrégat `Space` se charge incomplet, et hors de sa souveraineté (ticket `375-l-agregat-space-se-charge-incomplet-et-hors-souverainete`)
- **K12-B14** — L'ajout par un administrateur devient une commande du domaine (ticket `376-l-ajout-par-un-administrateur-devient-une-commande-du-domaine`)
- **K12-B15** — Le dépôt sait chercher dans l'annuaire de la plateforme (ticket `377-le-depot-sait-chercher-dans-l-annuaire-de-la-plateforme`)
- **K12-B16** — Ajouter un membre, côté applicatif (ticket `378-ajouter-un-membre-cote-applicatif`)
- **K12-B17** — `auth` sait créer un compte sans mot de passe (ticket `379-auth-sait-creer-un-compte-sans-mot-de-passe`)
- **K12-B18** — Le widget de création de compte, fourni par `auth` (ticket `380-le-widget-de-creation-de-compte-fourni-par-auth`)
- **K12-B19** — L'onglet Ajout direct : le cadre, la recherche, les candidats (ticket `381-l-onglet-ajout-direct-le-cadre-la-recherche-les-candidats`)
- **K12-B20** — Ajouter un coach déjà inscrit (ticket `382-ajouter-un-coach-deja-inscrit`)
- **K12-B21** — Créer un compte et ajouter (ticket `383-creer-un-compte-et-ajouter`)
- **K12-B22** — L'onglet Ajout direct sous Playwright (ticket `384-l-onglet-ajout-direct-sous-playwright`)

## Hors périmètre

Voir la section « Ce que l'épic ne couvre pas » de `initiative.md`, s'il y en a une.
