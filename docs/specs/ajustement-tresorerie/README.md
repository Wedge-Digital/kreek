# Ajuster la trésorerie d'une équipe

**Épic :** aucune pour l'instant · **Maquette :**
`assets/rawpages/html/app-team-treasury-admin.html`

## La fonction

Un commissaire d'espace crédite ou débite la caisse d'une équipe, et le
mouvement s'inscrit au relevé comme les autres — daté, motivé, signé.

Aujourd'hui, une décision qui touche à l'argent n'a nulle part où s'écrire :
compenser un forfait subi, sanctionner, rattraper une recette mal saisie dans un
rapport déjà corrigé. Il ne reste qu'à falsifier un rapport de match, ou à ne
rien faire.

C'est la fonction des **points de classement manuels**
(`docs/specs/points-classement-manuels/`) transposée à l'argent, et la plupart
de ses règles se transposent avec elle — sauf celles que la nature d'une caisse
change, notées à leur place.

## Les pages

| Page | État |
|---|---|
| `panneau-d-ajustement/` | phases 1 à 4 faites |

## Ce que la fonctionnalité suppose déjà acquis

**Le grand livre et son relevé.** `teams__treasury_ledger` porte `direction`,
`amount_kpo`, `reason`, `balance_after_kpo` et `occurred_at` ; l'onglet
Trésorerie le lit et l'affiche depuis les cartes 434 à 437. Il n'y a ni table à
créer, ni projection à écrire — un neuvième motif et un écran de saisie.

**Les huit motifs** vivent dans `teams/domain/treasury.rs`. Le fichier dit
lui-même que trois endroits sont à toucher pour en ajouter un — `ALL`, `as_str`
et la `garde_d_exhaustivite` du module de tests — et que ce dernier est le seul
mécanisme qui les relie.

**La règle d'admin.** `SpacePermissions::is_admin()` est déjà la garde du renvoi
d'équipe, et `garde_action_equipe.rs:20-25` pose la frontière : « La fiche
d'équipe, sa trésorerie et ses matchs se lisent par tout le monde — la carte 500
y retire les boutons, pas la page. » L'admin d'espace **exclut délibérément le
propriétaire** de l'équipe.

**L'aiguillage d'onglets.** La route de l'onglet rend toujours la page entière,
et c'est la barre d'onglets qui extrait `#team-tab-zone` (carte 484,
`team_detail.rs:453-470`).

## Ce que la fonctionnalité ne couvre pas

- **Annuler un ajustement.** Le relevé dérive de l'event store : une ligne
  écrite ne s'efface pas. Une erreur se corrige par un ajustement de sens
  opposé, et les deux lignes restent lisibles. C'est la différence avec les
  points manuels, dont la table à part autorise un `DELETE`.
- **Un journal des actions d'admin.** Le relevé *est* la trace, puisqu'il porte
  l'auteur et le motif. Un second écran qui les rassemblerait par commissaire
  est un autre besoin, qui n'a pas été exprimé.
- **La valeur d'équipe.** Elle ne bouge pas : `team_value.rs` est une somme de
  joueurs et de staff, où la trésorerie n'entre pas.
