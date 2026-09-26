# Le logo d'équipe n'a pas de porte d'entrée dans le domaine

**Priorité : moyenne**
**Dépend de :** rien ; bloque les cartes 509-511
**Fichiers :** `src/app/teams/domain/team.rs`,
`src/app/teams/domain/tests/` (nouveau ou existant selon l'organisation actuelle)

## Le défaut

`TeamDomainEvent::LogoChanged { logo_url: String }` existe déjà (`team.rs:245`),
et son application est déjà câblée (`apply()`, `team.rs:740` :
`self.logo_url = Some(logo_url.clone())`). Mais **aucune méthode publique ne le
produit** : `grep LogoChanged src/app/teams` ne remonte que sa déclaration et
son application, jamais une émission. L'agrégat n'a aucun moyen de dire
« change ton logo ».

C'est le même défaut de fabrication que documenté dans CLAUDE.md, section
« Les deux bouts de la chaîne » : un événement défini, jamais émis — le bras
du publisher est mort, et personne ne s'en aperçoit tant que rien ne cherche à
l'émettre.

## La décision

Ajouter une méthode de commande sur `Team`, sur le moule des méthodes
existantes (`dismiss`, `reject_enrollment`, etc. — event sourcing strict :
la méthode ne mute rien directement, elle retourne l'événement, `apply()`
s'en charge déjà) :

```rust
pub fn change_logo(&self, logo_url: CloudinaryImage) -> Result<TeamDomainEvent, DomainError> {
    Ok(TeamDomainEvent::LogoChanged { logo_url: logo_url.into_inner() })
}
```

Le type d'entrée est `CloudinaryImage` (`src/app/shared_kernel/identity/ids.rs`),
pas un `String` nu — cohérent avec la règle CLAUDE.md sur l'interdiction des
primitifs nus côté écriture, et avec le smart constructor déjà utilisé pour le
logo de compétition et le logo de création d'équipe.

### Décision actée — retrait du logo permis

Le retrait du logo (retour aux initiales) est autorisé. Signature retenue :

```rust
pub fn change_logo(&self, logo_url: Option<CloudinaryImage>) -> Result<TeamDomainEvent, DomainError> {
    Ok(TeamDomainEvent::LogoChanged { logo_url: logo_url.map(CloudinaryImage::into_inner) })
}
```

`TeamDomainEvent::LogoChanged` porte aujourd'hui `logo_url: String` (non
optionnel, `team.rs:245`) — **cette carte inclut donc le changement du type du
champ en `Option<String>`**, et l'ajustement du bras `apply()` (`team.rs:740`,
`self.logo_url = logo_url.clone()` au lieu de `Some(logo_url.clone())`). Un
changement de forme d'un événement déjà persisté : vérifier si des événements
`LogoChanged` existent déjà en base avant ce changement (improbable, puisque
jamais émis à ce jour — à confirmer par une requête sur `team_event_store`) ;
si aucun n'existe, pas de migration de données nécessaire, seulement le
changement de type Rust/serde.

Pas de garde métier au-delà de la validité de l'URL (`CloudinaryImage::try_new`
côté `Some`) : pas de vérification de phase de jeu, pas de restriction
temporelle — le logo reste modifiable à tout moment du cycle de vie de
l'équipe, y compris `Dismissed` ou `OffSeason`.

## Ce que la carte ne fait pas

- Elle ne câble ni route ni use case — seulement la méthode domaine et son
  test.
- Elle ne touche pas à la persistance de l'event store ni à la projection.

## Checklist

- [ ] Vérifier qu'aucun `LogoChanged` n'existe déjà dans `team_event_store` avant de changer le type du payload
- [ ] `TeamDomainEvent::LogoChanged { logo_url: Option<String> }` (changement de forme)
- [ ] `apply()` ajusté : `self.logo_url = logo_url.clone()` (retire `Some(...)`)
- [ ] `Team::change_logo(&self, logo_url: Option<CloudinaryImage>) -> Result<TeamDomainEvent, DomainError>`
- [ ] Test unitaire : `change_logo(Some(...))` produit `LogoChanged { logo_url: Some(...) }`
- [ ] Test unitaire : `change_logo(None)` produit `LogoChanged { logo_url: None }` (retrait)
- [ ] Test unitaire : rejeu de chaque variante met à jour `team.logo_url` en conséquence (couvre `apply()`, déjà écrit mais jamais exercé par ce chemin)
- [ ] `make lint`, `make test`
