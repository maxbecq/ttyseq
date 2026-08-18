# Stratégie de test

> Détaille le §4 de [dev-workflow.md](dev-workflow.md). Décrit le principe fondateur,
> les trois étages de test automatisés, ce qui ne s'automatise pas et comment on le
> couvre quand même, et les conventions pratiques.

## 1. Principe fondateur : un engine pur, un temps injecté

L'engine séquenceur ne possède pas d'horloge et ne fait aucun I/O. Son API a
conceptuellement cette forme :

```rust
impl Engine {
    fn handle(&mut self, cmd: EngineCommand);
    /// Avance le temps de `frames` échantillons et pousse
    /// les événements produits dans `out`.
    fn advance(&mut self, frames: u64, out: &mut Vec<EngineEvent>);
}
```

En production, `advance` est appelé par le thread audio à chaque buffer. En test,
c'est le test qui appelle. Conséquences :

- tout le comportement séquenceur est testable **sans carte son, sans thread,
  en microsecondes** ;
- les tests sont **déterministes et rejouables** : un bug de transition se reproduit
  à l'échantillon près, à chaque exécution ;
- le cœur reste une fonction d'état lisible, pas un enchevêtrement de callbacks —
  ce qui sert aussi les règles temps réel (pas d'allocation, pas de syscall,
  pas de panique dans ce chemin).

## 2. Étage 1 — Tests unitaires sur la logique pure

**Quoi** : l'arithmétique musicale et les calculs de bornes, isolément.

- Conversions `bars/beats ↔ frames` selon tempo et signature. Point d'attention :
  à 128 BPM / 48 kHz un beat tombe juste (22 500 frames), à 127 BPM non.
  L'implémentation doit utiliser un accumulateur d'erreur (pas d'arrondi par beat),
  et des tests de **dérive cumulée** le vérifient : après 1 000 mesures, l'erreur
  totale reste < 1 frame.
- Points de bouclage : `length - tail_length`, sections d'une mesure,
  cas dégénérés (`tail_length == length`).
- Cas limites : signatures irrégulières (7/8), tempos non entiers.

**Où** : `#[cfg(test)]` dans le module concerné, au plus près du code.

**Property-based testing** : quelques tests `proptest` sur les conversions
(ex. « pour tout tempo et toute durée, convertir puis reconvertir ne dérive jamais
de plus d'une frame »). Excellent rapport effort/bugs sur cette arithmétique.

## 3. Étage 2 — Tests de scénario sur l'engine (le cœur de la stratégie)

**Quoi** : chaque décision actée de [data-model.md §5](spec/data-model.md#5-décisions-actées-)
devient un ou plusieurs tests nommés d'après elle. Un simulateur enrobe l'engine :

```rust
let mut sim = Simulator::new(project);
sim.play();
sim.advance_bars(17);                  // dans la queue du LoopTail
sim.stop();                            // → sortie de boucle programmée
sim.advance_to_section_end();
assert_eq!(sim.current_section(), "Outro");   // enchaînement sans silence
```

Couverture initiale, dérivée des décisions :

| Décision | Tests |
|---|---|
| Stop hybride (data-model §3.1) | stop hors boucle coupe net ; re-Play reprend à la section suivante ; re-Play après stop sur la dernière section d'une song → song suivante ; premier stop en `LoopFull` → sortie en fin de passage ; second stop pendant le dernier passage → coupe nette ; même trio pour `LoopTail` |
| Boucles illimitées | 50 passages de `LoopFull` sans avancement ; `LoopTail` reboucle au point `length - tail_length` |
| Clips coupés net | clip plus long que la section → silence exactement à la frontière ; `OneShot` plus court → silence puis rien |
| Fin de song via `on_end` | dernière section `Advance` → song suivante démarre, tempo changé à la frontière exacte ; dernière section `Stop` → attente de Play |
| Track sans clip | silencieuse, les autres jouent |

Deux exigences de conception que ces tests imposent dès le départ :

1. **Frontières en milieu de buffer.** `advance(512)` peut contenir une fin de
   section à la frame 300. Les tests avancent volontairement par tailles non
   alignées (512, 480, 1 frame) pour vérifier que les transitions sont
   sample-accurate, pas « au prochain buffer ».
2. **Les événements comme contrat.** On n'inspecte pas l'état interne : on affirme
   la séquence d'`EngineEvent` émise — exactement ce que verront le TUI et les
   clients du protocole.

**Où** : dans `tests/` (tests d'intégration), donc via l'API publique. Effet
secondaire voulu : l'API publique doit être expressive avant même qu'un client existe.

**Fixtures** : projets construits en code via des builders
(`project().song(...).section(...)`) tant que la sémantique bouge — cf.
[dev-workflow.md §5](dev-workflow.md).

## 4. Étage 3 — Rendu offline

**Quoi** : une fonction `render(project, n_frames) -> Vec<f32>` qui fait tourner
engine + mixage sans carte son. Les fixtures audio sont des sinusoïdes générées
par les helpers de test — une fréquence différente par clip, pour savoir *qui*
joue à chaque instant.

Assertions **structurelles** d'abord :

- silence strict après une coupe de fin de section, dès la frame calculée ;
- continuité au point de bouclage (le contenu à `loop_point + n` égale le contenu
  à `start + n` — pas de clic) ;
- RMS non nul sur les fenêtres où un clip doit jouer, nul ailleurs.

Un golden file (hash du rendu) peut s'ajouter ensuite comme filet anti-régression,
mais une assertion structurelle qui échoue dit *quoi* est cassé ; un hash dit
seulement *que* c'est cassé.

## 5. Ce qui ne s'automatise pas (et comment on le couvre)

- **Callback cpal, latence, jitter** : mesure manuelle sur cible, checklist écrite
  (cf. `doc/spikes/`). Un binaire smoke-test (`ttyseq --check-audio` : 2 s de sinus)
  valide une installation.
- **Règles temps réel** : la crate `assert_no_alloc` fait paniquer les builds de
  dev/test si le chemin audio alloue — installée dans le callback dès qu'il existe.
  Le « pas de panic » est couvert indirectement : le cœur pur étant massivement
  testé aux étages 1-2, les `unwrap` n'ont pas de raison d'y exister.
- **Parsing TOML** (quand on y sera) : round-trip serde + fixtures d'erreur —
  notamment le message « sample rate 44100 trouvé, 48000 attendu »
  (data-model §5, décision 15), qui a son test dédié.
- **TUI** : hors scope test pour l'instant (`ratatui` a un `TestBackend` si besoin
  plus tard). Le TUI étant un client passif du protocole, l'essentiel de sa logique
  est déjà couvert par les événements testés à l'étage 2.

## 6. Conventions pratiques

- `cargo test` reste **sous la seconde** (étages 1-2 en millisecondes) : on le lance
  à chaque sauvegarde.
- CI GitHub Actions minimale (fmt, clippy, test) posée avec le squelette Cargo.
- **Un test par décision, nommé d'après elle**
  (ex. `stop_during_loop_full_exits_at_end_of_pass`). Si un test de scénario gêne,
  c'est qu'on est en train de changer une décision de spec — et ça doit se voir.
