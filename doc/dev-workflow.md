# Processus de travail

> Comment nous travaillons sur ttySeq : répartition des rôles entre Max et l'assistant,
> cycle de décision, stratégie de test. Ce document décrit le **processus**, pas le produit —
> les specs restent dans `doc/spec/`. Référencé par `CLAUDE.md`.

## 1. Répartition des rôles

Objectif : que Max comprenne réellement le code, pas seulement qu'il existe.

- **Cœur métier — Max écrit, l'assistant guide et relit.**
  Modèle de données, machine à états de session (sections, `on_end`, transport).
  On y va lentement : sketch discuté et validé avant d'écrire le code.
- **Plomberie — l'assistant écrit, Max relit.**
  Parsing/sérialisation, glue cpal/midir, TUI, builders de test, CI.
  Chaque PR de plomberie inclut un guide de lecture (walkthrough) dans sa description.

La frontière peut bouger d'un commun accord, jamais implicitement.

## 2. Cycle d'une décision de conception

1. La question est posée avec des **options et une recommandation argumentée** —
   jamais tranchée unilatéralement par l'assistant.
2. La décision se prend **dans le dialogue**.
3. Elle est écrite dans la spec : la question passe de « ouverte ❓ » à « actée ✅ »,
   et les références résiduelles (exemples, roadmap, commentaires) sont nettoyées.
4. Dès que le code concerné existe, **un test nommé d'après la décision** la matérialise.
   Les tests de scénario sont la version exécutable des décisions de spec.

## 3. Discipline PR

- `main` est protégée ; tout passe par PR.
- **Une PR = un concept** (« les durées musicales », « la machine à états de section »),
  petite et réellement relue.
- Conventional Commits. Pas de `Co-Authored-By` ni de mention d'outil dans les commits et PR.

## 4. Stratégie de test — trois étages

> Détail complet (exemples, couverture par décision, outillage) : [test-strategy.md](test-strategy.md).
> Les validations matérielles se font en spikes documentés dans `doc/spikes/`.

L'engine séquenceur est **pur** : il ne possède pas d'horloge, ne fait aucun syscall ;
on lui fournit le temps écoulé et les commandes, il répond par des événements.
En production c'est le callback audio qui fait avancer le temps ; en test, c'est le test.

1. **Tests unitaires sur la logique pure** (le gros du volume) : arithmétique musicale
   (bars/beats ↔ frames), points de bouclage, bornes de sections.
2. **Tests de scénario sur l'engine** : instancier un projet, envoyer Play/Stop,
   avancer le temps simulé, affirmer la séquence d'événements émise.
   Déterministes, rejouables, sans matériel.
3. **Rendu offline** (quand l'audio existe) : rendre un projet dans un buffer sans carte
   son et comparer à une référence (golden file). Teste mixage et découpage à
   l'échantillon près. La latence et la stabilité réelles se mesurent à la main sur cible.

## 5. Fixtures de test

- **Tant que la sémantique se stabilise** : projets construits en code Rust via des
  builders (`project().song(...).section(...)`), pas de fichiers sur disque.
- **Une fois le format de fichier stabilisé** : un projet fixture sur disque
  (`tests/fixtures/`) couvrant les quatre `on_end`, avec des fichiers audio **générés**
  par les helpers de test (sinusoïdes courtes) — pas de binaires commités.
