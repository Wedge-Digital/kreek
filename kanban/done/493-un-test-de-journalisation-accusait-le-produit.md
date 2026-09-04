# Un test de journalisation accusait le produit

**Priorité : haute** — la CI de `demo` est rouge
**Dépend de :** rien · **Corrige un test de la carte 486** · **Renumérotée depuis 492**, une autre session ayant pris ce numéro 29 minutes plus tôt · **Sans épic**
**Trouvée par :** la CI, sur le push de la carte 490

## Le symptôme

```
---- unpublish_match_report_use_case::tests::un_refus_d_eligibilite_est_journalise ----
assertion `left == right` failed: une ligne et une seule : []
  left: 0   right: 1
```

Le test de la carte 486 : il vérifie qu'un refus de dépublication laisse une
trace. La capture n'a **rien reçu**.

Le job « Tests unitaires » échoue seul ; qualité, audit et e2e passent. Les trois
tests du cookie livrés par la carte 490 sont verts — **ce n'est pas elle qui
casse**.

## Ce qui n'a pas été reproduit

Quinze exécutions de la suite complète en local : cinq à dix-huit fils, trois à
deux fils, trois à quatre, plus une passe en mono-fil dans l'ordre alphabétique.
**Toutes vertes.**

La machine de développement a dix-huit cœurs ; le runner en a quatre, et met
232 s là où il en faut 25 ici. La fenêtre n'est pas la même, et je n'ai pas su
la rouvrir.

## L'explication retenue, et son statut

`tracing` met en cache **l'intérêt de chaque point d'émission**. Ce cache est
*global*, alors que `capture_sous_le_filtre_de_production` ne pose qu'un abonné
*de thread*, par `set_default`. Un point d'émission évalué la première fois
depuis un thread sans abonné peut y rester marqué comme sans intérêt — et la
capture ne reçoit alors rien, sans que rien ne le signale.

C'est ce que `tracing` documente, et le remède qu'il prescrit est
`rebuild_interest_cache()`, appelé après la pose de l'abonné.

**Ce n'est pas une cause démontrée.** C'est la seule qui explique le symptôme, et
elle est écrite comme telle dans le code.

## La garde qui compte davantage

Le message d'échec accusait le use case. Il était faux : le produit journalisait
peut-être très bien, c'est l'instrument qui était muet. Un test qui se trompe de
coupable coûte plus cher qu'un test qui échoue.

`capture_sous_le_filtre_de_production` émet donc désormais une **ligne témoin**
et vérifie qu'elle arrive. Si la capture ne reçoit rien, l'échec se produit dans
`capture_journal.rs` et dit ce qu'il en est — au lieu de laisser croire que le
use case ne journalise pas.

Le témoin porte une cible `kreek::`, sans quoi il ne franchirait pas le filtre de
production et prouverait le contraire de ce qu'on lui demande.

## Ce que la carte ne fait pas

**Elle ne rend pas le test déterministe par construction.** Tant que la capture
passe par un abonné de thread, elle dépend d'un état global de `tracing`. Si le
symptôme revient malgré `rebuild_interest_cache`, la sortie est de tester la
décision plutôt que son émission : une fonction pure qui rend ce qu'il faut
journaliser, et un seul test — posé une fois — pour vérifier que la ligne
franchit le filtre.

**Elle ne touche pas au produit.** `journaliser_le_refus` est inchangé.

## Le second blocage : un `goto` plus rapide que le POST

`make e2e` échouait aussi sur `test_renommer_une_competition_depuis_les_parametres`,
que la carte 489 avait pourtant déjà touché en y posant l'attente de câblage.
Cinq exécutions du test seul : vertes. Dans la suite complète, sous charge :
rouge.

**L'attente de câblage n'était pas la seule chose qui manquait.** Après le clic,
rien n'attendait que le POST ait abouti :

```python
page.click(…)
expect(input).to_have_value(nouveau)   # creux : `fill` a déjà mis cette valeur
page.goto(admin_url + "/settings")     # part parfois avant l'écriture
expect(input).to_have_value(nouveau)   # échoue, et accuse le produit
```

L'assertion intermédiaire est creuse **par nature** : elle ne distingue pas un
serveur qui a répondu d'un champ resté tel qu'on l'a rempli. Le clic est
désormais entouré d'un `expect_response` sur le POST, ce qui la rend honnête et
ferme la course avec le rechargement.

C'est le même motif que la carte 486, à un écran près : **un test qui n'attend
pas ce qu'il croit attendre, et dont le message accuse le produit.**

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_refus_d_eligibilite_est_journalise` | inchangé — c'est lui qu'on stabilise |
| `une_depublication_reussie_ne_journalise_pas_de_refus` | inchangé |
| la ligne témoin | **falsifiée** : émise hors `kreek::`, l'échec se produit dans `capture_journal.rs`, pas dans le test du use case |

## Checklist

- [x] `rebuild_interest_cache()` après la pose de l'abonné
- [x] `tracing-core` en dépendance directe, avec son motif
- [x] La ligne témoin, falsifiée
- [x] `make lint`, `make test` (1632), `make check-arch` (17 axes), `make audit`
- [x] `make e2e` — 352 passés, 0 échec
- [ ] **La CI verte** — c'est le seul verdict qui vaut, l'échec n'étant pas reproductible ici
