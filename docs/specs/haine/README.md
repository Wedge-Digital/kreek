# La Haine — Progression

Un joueur blessé peut gagner la **Haine**, un trait qualifié par un mot-clef
(« Haine : Nain »). C'est une compétence acquise **gratuite** : elle ne déplace
ni la valeur du joueur, ni celle de l'équipe.

## Maquettes (Phase 1 ✅)

Validées et commitées (`fde2690`) :

| Écran | Fichier |
|---|---|
| Gain de la Haine à la saisie des actions | `assets/rawpages/html/app-match-report-step3-haine.html` |
| Mots-clefs du poste dans la fiche d'équipe | `assets/rawpages/html/app-team-detail-keywords.html` |
| Mots-clefs et Haines sur la fiche joueur | `assets/rawpages/html/app-player-detail-haine.html` |

## Progression par page

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| Saisie des actions — gain de la Haine | ✅ | | | | | | |
| Fiche d'équipe — mots-clefs | | | | | | | |
| Fiche joueur — mots-clefs et Haines | | | | | | | |

## Décisions déjà prises

**La mise à plat plutôt qu'une compétence paramétrée.** Trente-huit compétences
`HAINE_<MOT_CLEF>` en catégorie `TRAITS`, plutôt qu'une compétence unique portant
un paramètre. Le corpus s'y prête déjà : la catégorie `TRAITS` existe dans
`skill_cat_fr.json`, et **elle n'est accessible à aucun poste** — vérifié sur les
`primaryAccess` et `secondaryAccess` de tout le corpus. Deux conséquences
gratuites : les Haines n'apparaissent pas dans le sélecteur de compétences, et
`resolve_skill_cost` les refuse par `CategoryNotAccessible`, donc elles ne sont
pas achetables en SPP.

**Trois blessures sur cinq la donnent** : Amoché, Blessure Sérieuse, Séquelle.
Ni une Commotion, ni une Mort.

**Le trait est gratuit.** Pas de valeur en kPo, donc aucun recalcul de valeur
d'équipe, et rien à faire du côté de `teams`. Suivant le précédent posé par la
customisation, l'événement ne portera **pas** de champ de valeur à zéro : « il
n'existe pas, il ne vaut pas zéro ».

**Pas de doublon** : un joueur ne gagne pas deux fois le même mot-clef. Le cumul
de Haines différentes, lui, n'est pas borné.

**On suit le mécanisme d'impact de match existant**, sans en créer un second.

**Supprimer l'action supprime la Haine.**

## Ce que la fonctionnalité ne couvre pas encore

**L'effet de la Haine en jeu.** Les lignes de roster portent désormais leurs
`keywords` (`"keywords": ["BLOCKER", "HUMAN"]`), ce qui rend la comparaison
possible — mais aucune règle d'effet n'est spécifiée à ce stade.
