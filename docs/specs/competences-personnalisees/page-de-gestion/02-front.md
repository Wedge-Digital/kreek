# Page de gestion · Phase 2 : architecture front

**Maquette** : `assets/rawpages/html/app-custom-skills.html`

## Où la page vit — et le piège qu'il faut éviter

L'écran appartient à l'administration d'un espace, aux côtés des membres. Mais
**il ne peut pas être un onglet de `space-admin.html`.**

`spaces` est un **BC extractible** (`check-arch.sh:60`, `EXTRACTABLE_BCS="auth
spaces"`). Le `CLAUDE.md` lui interdit de référencer un autre BC — ses routes
comprises :

> Un BC prévu pour être réutilisé dans un autre projet n'utilise **que ses
> propres `Routes`**, jamais `AppRoutes`.

Un onglet « Compétences » dans `space-admin.html` obligerait `spaces` à connaître
`references`. L'axe 9 de `check-arch` le refuserait, et **avec raison** : copier
`spaces/` dans un autre projet emporterait une dépendance vers un BC qui n'y
serait pas.

**La page est donc autonome**, servie par `references`, sous
`/app/{space_id}/admin/skills`. L'administration d'espace peut y mener par un
lien — un lien sortant est une `String` que l'hôte injecte, pas un import.

C'est le même arbitrage que pour les rosters personnalisés (épic E10), et les
deux écrans se rejoindront naturellement dans une administration d'espace élargie
que `references` servira.

## Un assemblage à deux widgets

| Widget | Endpoint | Trigger | Écoute |
|---|---|---|---|
| `#cs-form` | `custom_skill_form` | `load` | `customSkillSelected from:body` |
| `#cs-list` | `custom_skill_list` | `load` | `customSkillsChanged from:body` |

**Deux événements, dans les deux sens** — et c'est ce qui distingue cet écran de
celui des points de classement manuels, où le formulaire n'écoutait rien.

```
POST créer / PUT modifier  → HX-Trigger: customSkillsChanged  → la liste se recharge
DELETE                     → HX-Trigger: customSkillsChanged  → idem
clic « Modifier le libellé » → customSkillSelected { skill_id } → le formulaire se charge
```

**Le formulaire écoute parce qu'il a deux modes** : création et édition. Cliquer
« Modifier le libellé » dans la liste doit le remplir — ce qu'aucun rechargement
de liste ne saurait faire.

### Pourquoi deux widgets et non un fragment

Comme pour les points manuels : le formulaire **garde son état** pendant qu'on
crée plusieurs compétences d'affilée. Un fragment unique le réinitialiserait à
chaque enregistrement.

## Le formulaire — deux modes, et un verrou partiel

| Mode | Nom | Description | Catégorie | Type |
|---|---|---|---|---|
| Création | libre | libre | libre | libre |
| Édition, **inemployée** | libre | libre | libre | libre |
| Édition, **employée** | libre | libre | **figée** | **figé** |

**Le verrou partiel est le cas que la conception n'avait pas prévu**, et il
mérite d'être dit : ni tout ouvert, ni tout fermé.

La ligne passe là où l'argent se trouve. Changer une compétence de `Standard` à
`Élite` la ferait coûter 10 kPo de plus et changerait son coût en SPP —
**rétroactivement**, pour des joueurs qui ont payé l'ancien prix. Même chose pour
la catégorie, qui décide de l'accès primaire ou secondaire, donc du barème.

Le nom et la description n'engagent rien : **une faute se corrige.**

### Un champ figé se transforme, il ne se grise pas

```
Catégorie   🔒 Agilité          ← un fait
Type        🔒 Élite            ← un fait
```

Et non un `<select>` désactivé. Griser inviterait à chercher comment réactiver
alors qu'il n'y a rien à réactiver — c'est le même principe que le bouton
Supprimer **absent** plutôt que désactivé sur les rosters.

### Le type dit ce qu'il coûte

Un segmenté « Standard / Élite », et choisir Élite affiche « +10 kPo à
l'achat ». C'est le seul champ à conséquence chiffrée (carte 387) : **la
conséquence se voit au moment de la décision**, pas quand un coach paie.

### L'aperçu de la pastille

La compétence sera lue sur une fiche de joueur, sous forme de pastille colorée
par sa catégorie. L'aperçu la montre **avec sa vraie teinte** — les sept de
`skill_category_css` : `type-general`, `type-agility`, `type-traits`…

La voir ici évite de découvrir après coup qu'elle se confond avec une autre.

### La description est un `textarea`

Au corpus, la médiane fait 32 caractères et le maximum 103 — mais une règle
maison sera souvent plus longue. Le compteur borne à 600.

L'indication dit ce que ce texte **est** : *« c'est ce texte que les coachs
liront pour appliquer la règle sur table »*. L'application ne l'appliquera
jamais.

## La liste

Une ligne par compétence : le nom, la pastille de catégorie, la marque Élite
s'il y a lieu, la description, et **le compteur d'usage qui décide des
actions**.

| Usage | Actions |
|---|---|
| zéro joueur | Modifier · **Supprimer** |
| au moins un | Modifier le libellé — et un badge « 🔒 Non supprimable » |

**Le badge dit ce qui est verrouillé**, pas « verrouillée » tout court :
justement, le libellé se modifie.

**Aucune section « compétences du règlement ».** Contrairement aux rosters, où
les deux listes cohabitaient : il y en a 43 au corpus de démonstration, bien
plus en production, et les lister ici n'aiderait personne — le sélecteur de
compétences les montre déjà là où on les choisit.

## Ce qui reste front

**Rien de neuf.** Le compteur de caractères, l'aperçu de pastille et la bascule
du type sont de l'état d'écran, dérivés des champs sans aller-retour. Le reste
est rendu par le serveur.

## Ce que la page ne fait pas

- **Aucun classement, aucun filtre** : une poignée de compétences par espace.
- **Aucun partage entre espaces.**
- **Aucune modération** du texte saisi.
- **Aucune duplication** d'une compétence du règlement pour la retoucher —
  comme pour les rosters, ce geste mérite sa propre décision.

## CSS

Une feuille, `pages/references-custom-skills.css`, portée par `.cs-page`, à
inscrire dans `src/web/css_bundle.rs` — l'axe 14 refuse toute feuille absente du
bundle.

**Les pastilles de catégorie ne sont pas réécrites** : `type-general`,
`type-agility` et les cinq autres existent dans `widgets/players-widget.css`.
Les redéfinir ferait deux jeux de teintes qui dériveraient — une compétence
verte ici, bleue sur la fiche du joueur.

**À trancher en phase 3** : les extraire dans un composant partagé, ou accepter
que la feuille de cette page en dépende.

## Règles métier

**Aucune à préciser.** Les six de la phase 1 couvrent la fonctionnalité.

Deux points restent pour la phase 3, tous deux techniques :

1. **Seize sites lisent le catalogue de compétences** — `list_skills`,
   `find_skill_by_uid`, `find_skill`. Tous devront voir les compétences de
   l'espace, et **aucun compilateur ne le signalera** pour ceux qui ne changent
   pas.
2. **Le catalogue est en mémoire**, chargé au démarrage. Les compétences
   d'espace posent le même problème que les rosters personnalisés (carte 441) —
   et probablement la même solution.
