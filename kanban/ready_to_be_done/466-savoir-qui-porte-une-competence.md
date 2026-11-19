# Savoir qui porte une compétence

**Épic :** E10 — Référentiels éditables · **Ordre :** 4 · **Dépend de :** 441
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/`
(`03-back.md`, `04-dtos.md`)

## Objectif

Répondre à « cette compétence est-elle employée ? ». C'est cette réponse qui
décide du verrou partiel et de la suppression.

## Le port

```rust
// references/ports.rs — créé par la carte 441
#[async_trait]
pub trait ISkillUsagePort: Send + Sync {
    /// Joueurs qui l'ont acquise **plus** postes qui la posent en compétence de
    /// base. Zéro autorise la suppression.
    async fn count_usages(&self, skill_uid: &str) -> Result<u32, String>;
}
```

## Deux comptes, pas un

Contrairement aux rosters — où la question n'avait aucune réponse et exigeait une
colonne neuve (carte 442) — **le premier compte est directement interrogeable** :

```sql
SELECT count(*) FROM players_proj
WHERE  acquired_skills @> jsonb_build_array(jsonb_build_object('skill_id', $1))
```

**Mais un joueur n'est pas le seul porteur.** Une compétence peut être une
compétence de base d'un poste, dans un roster personnalisé — et là aucune requête
sur `players_proj` ne la trouve : elle vit dans le JSONB `definition` de
`references__custom_rosters`.

```sql
SELECT count(*) FROM references__custom_rosters
WHERE  definition -> 'positions' @> …   -- l'uid dans les `skills` d'un poste
```

**Oublier le second laisserait supprimer une compétence qu'un roster pose**, et
le poste afficherait un uid mort — exactement le défaut de la carte 438.

C'est la dépendance vers la 441, qui crée cette table. **Les deux séries partent
ensemble** : le second compte n'est pas un repli à zéro qu'on rebranchera plus
tard, c'est la moitié de la réponse.

## Un joueur licencié compte

Un joueur mort, licencié ou retiré reste dans `players_proj`, et sa fiche reste
consultable. **Il compte comme porteur.**

Bloquer une suppression de trop coûte un libellé qui traîne ; l'inverse coûte une
fiche où un uid ne résout plus rien — le silence de la carte 438.

**À vérifier au moment d'écrire la requête** : c'est la projection qui a le
dernier mot sur ce qu'elle garde. Si elle purge les licenciés, la requête doit
aller les chercher ailleurs, pas se contenter du compte qu'elle rend.

## L'indisponibilité n'est pas un zéro

Le port rend `Result`. Traiter une erreur comme un zéro laisserait supprimer une
compétence que cent joueurs portent parce qu'une requête a échoué. **Le doute
ferme la porte** — mais seulement celle qu'il concerne (carte 467).

## L'adapter

`src/infrastructure/references/skill_usage_adapter.rs` — le seul à importer
`players`. Le BC `references` ne connaît que son trait.

**Une seule requête** additionnant les deux comptes, ou deux en `tokio::join!` :
les deux tables appartiennent à des BCs différents, mais l'adapter est justement
l'endroit où cette frontière se franchit légitimement.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `une_competence_que_personne_ne_porte_compte_zero` | le cas passant |
| `un_joueur_qui_l_a_acquise_compte_pour_un` | le premier compte |
| `un_poste_de_roster_personnalise_qui_la_pose_compte` | **le second, celui qu'on oublie** |
| `les_deux_comptes_s_additionnent` | un joueur et un roster font deux |
| `un_joueur_licencie_compte_encore` | la règle U8 |
| `une_competence_du_corpus_se_compte_aussi` | le port ne filtre pas sur le préfixe |

Le dernier mérite son motif : le port répond pour n'importe quel uid. C'est ce
qui permettra un jour de dire « 42 joueurs ont Esquive » sans rien changer.

## Checklist

- [ ] `ISkillUsagePort` dans `references/ports.rs`
- [ ] `infrastructure/references/skill_usage_adapter.rs`, les **deux** comptes
- [ ] Le sort des joueurs licenciés **vérifié** dans `players_proj`, pas supposé
- [ ] Instanciation dans `main.rs`, injection dans le contexte
- [ ] Les six tests
- [ ] `make lint && make test && make check-arch`
