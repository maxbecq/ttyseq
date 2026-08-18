# CLAUDE.md

## Langue

- **Dialogue avec l'utilisateur : français.**
- **Code, commentaires, identifiants, messages de commit, documentation externe (README, doc publique) : anglais.**
- Les specs internes sous `doc/spec/` sont actuellement en français ; ne pas les traduire sans demande explicite.

## Vision

ttySeq est un séquenceur musical hybride **audio / MIDI / CV** en Rust, piloté au terminal,
conçu pour la **performance live** de musique électronique sur matériel à faibles ressources
(Raspberry Pi, vieux ordinateurs). C'est un **instrument de scène, pas un DAW** : la composition
se prépare ailleurs, ttySeq exécute de façon fiable et légère.

## État du projet

Phase de **spécification terminée, implémentation à démarrer**. Toutes les questions du
modèle de données sont tranchées (cf. `data-model.md §5`). Le code produit n'existe pas
encore ; prochaine étape : squelette Cargo, types du modèle de données et durées musicales,
puis machine à états de session — selon les rôles et la stratégie de test définis dans
`doc/dev-workflow.md` et `doc/test-strategy.md`.

La référence fait foi : `doc/spec/spec.md` (vue d'ensemble) et `doc/spec/data-model.md`
(hiérarchie de données, comportements, décisions actées).

Lire ces deux fichiers avant toute proposition d'architecture ou de code.

Acquis validés par le spike — code de référence figé dans `spikes/audio-path/`
(cf. son README ; on y puise, on ne le fait pas évoluer ; à exclure du futur workspace Cargo) :

- Le chemin audio temps réel (cpal + ring buffer lock-free `rtrb`) : 60 min / 0 underrun
  sur Raspberry Pi 4.
- Le pilotage de l'écran SSD1322 du Norns Shield (boutons et encodeurs pas encore testés).

Spike différé, protocole prêt : validation Babyface Pro × Raspberry Pi en mode
Class Compliant (`doc/spikes/babyface-raspi.md`).

## Architecture (cible)

- **Engine séquenceur** entouré d'une couche protocole ; un ou plusieurs **clients** s'y branchent
  (TUI embarqué, CLI, intégrations live coding). Voir `doc/spec/spec.md §3.3`.
- Vocabulaire de messages unique `EngineCommand` / `EngineEvent`, utilisé en interne (MPSC, sans
  sérialisation) comme en externe (socket Unix, OSC, avec `serde`). Garder les messages sous forme
  de **données pures** : pas de closures, pas de pointeurs partagés.
- Modes de lancement d'un binaire unique : `ttyseq` (engine + TUI embarqué, défaut solo),
  `ttyseq daemon` (headless), `ttyseq attach` (client TUI distant), `ttyseq <subcommand>` (one-shot CLI).
- Transports externes : socket Unix toujours ouvert (`$XDG_RUNTIME_DIR/ttyseq.sock`, pas de surface
  réseau) ; OSC sur UDP **désactivé par défaut**, loopback only quand activé.
- La sync tempo à l'échantillon (MIDI Clock, etc.) est un **sujet distinct** de cette IPC de contrôle.

## Règles temps réel (NON NÉGOCIABLES)

Dans le callback audio (chemin temps réel) :

- **Pas d'allocation** mémoire.
- **Pas de syscall.**
- **Pas de `panic!`** ni de `.unwrap()` / `.expect()` susceptibles de paniquer.
- Communication avec le reste du système via canaux et ring buffer lock-free (`rtrb`) uniquement.

Toute proposition qui viole une de ces règles dans le chemin audio doit être signalée et corrigée
avant d'aller plus loin.

## Conventions

- **Conventional Commits** (`feat:`, `fix:`, `docs:`, `refactor:`…).
- **Workflow par PR** : `main` est protégée, pas de push direct dessus.
- Maintenir le **numéro de version et la date** dans l'en-tête des specs lors d'un changement de fond.
- Lors d'une modification de spec, **nettoyer proactivement les références résiduelles**
  (commentaires, roadmap, exemples) qui pointent vers l'ancien état.

## Méthode de travail

- **Processus de collaboration détaillé** (répartition des rôles, cycle de décision,
  stratégie de test, fixtures) : `doc/dev-workflow.md`. Le lire avant de proposer du code.
- **Spike avant abstraction** : valider par un prototype qui tourne avant de figer des interfaces.
  Une mauvaise abstraction verrouillée tôt coûte cher.
- **Différer le découpage en crates** jusqu'à ce que les coutures émergent du code qui tourne.
- Itérer : valider les décisions dans le dialogue avant de les écrire dans les specs.
- Instructions d'édition courtes et précises attendues ; appliquer les changements de spec
  directement sur la branche de PR active.

## Points non tranchés (ne pas inventer de réponse)

À l'état actuel des specs, restent ouverts — ne pas présupposer un choix :

- **Licence** : GPL v3 vs MIT (cf. `spec.md §11`).
- **Format de sérialisation wire** externe : JSON vs MessagePack vs Bincode (cf. `spec.md §3.3.4`).

Les questions du modèle de données (Stop, Loop, offset des clips, transitions entre songs,
sample rate, tempo) et le format projet/config (TOML) ont été tranchés le 18 août 2026 —
cf. `doc/spec/data-model.md §5`.

Si une décision sur un de ces points est nécessaire pour avancer, **le signaler explicitement**
et proposer des options, sans trancher unilatéralement.

## Stack technique

Rust. `cpal` (audio), `midir` (MIDI), `ratatui` (TUI), `rtrb` (ring buffer lock-free),
`thread-priority`, `serde`, `symphonia` (décodage audio). IPC externe : socket Unix natif,
OSC via `rosc` (à confirmer). Cibles : Raspberry Pi (dont Fates), macOS Apple Silicon (dev).
