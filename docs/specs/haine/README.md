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
| Saisie des actions — gain de la Haine | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ **399-404** |
| Fiche d'équipe — mots-clefs | — | — | — | — | — | — | ✅ **405** |
| Fiche joueur — mots-clefs et Haines | — | — | — | — | — | — | ✅ **405** |

Ces deux pages **ne passent pas par le workflow** : il ne s'agit que d'afficher
des informations déjà présentes, sur des écrans qui existent. La carte 405 les
couvre à elle seule, et répare au passage le `match` des catégories de
compétences, où `MUTATIONS` est testé au singulier et `DEVIOUS` absent — les deux
s'affichent aujourd'hui avec la couleur du général.

## Décisions déjà prises

**La mise à plat plutôt qu'une compétence paramétrée.** Une compétence par
mot-clef haïssable — **trente**, en catégorie `TRAITS` — plutôt qu'une compétence
unique portant un paramètre. Les huit mots-clefs de rôle (Blitzer, Bloqueur,
Receveur, Coureur, Lanceur, Trois-quart, Gros Bras, Spécial) ne se haïssent pas :
on hait une espèce, pas un poste. Le corpus le dit par
`league_hate_selectable`, et porte le lien vers la compétence par
`hate_skill_uid` — **aucune convention de nommage, aucune liste dans le code**. Le corpus s'y prête déjà : la catégorie `TRAITS` existe dans
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

**Aucune gestion des doublons** : un joueur peut recevoir deux fois le même
mot-clef, c'est au coach de ne pas le faire. Le panneau de saisie ne connaît pas
les Haines déjà acquises — elles vivent dans `players` — et s'en passer évite une
consultation inter-BC, une règle de refus et ses tests. Le cumul de Haines
différentes n'est pas borné non plus.

**On suit le mécanisme d'impact de match existant**, sans en créer un second.

**Le quatrième mode d'acquisition s'appelle `Injury`.** Le coach répond oui ou
non puis choisit son mot-clef : rien d'automatique. Les trois modes existants
nomment la façon d'obtenir — le coach a choisi, le dé a choisi, un commissaire a
posé — et la quatrième case de cette série est « à la suite d'une blessure ».

**La Haine d'un journalier reste dans le rapport de match.** Aucun agrégat
`players` n'existe pour un joueur temporaire, et rien ne le relie au joueur qu'on
engagerait ensuite.

**Supprimer l'action supprime la Haine.**

## Ce que la fonctionnalité ne couvre pas encore

**L'effet de la Haine en jeu.** Les lignes de roster portent désormais leurs
`keywords` (`"keywords": ["BLOCKER", "HUMAN"]`), ce qui rend la comparaison
possible — mais aucune règle d'effet n'est spécifiée à ce stade.
