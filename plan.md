# Plan

## Goal
Build a single-page quiz-show website ("Quiz Zoo") that serves a
question from a pool of ~550 vocabulary words, tracks the current
round, scores the answer, and lets the user start a new round.

## Scope
- **Home screen:** brand lockup, headline, "Start Quiz" button, small
  stat line (words in pool, round, score).
- **Quiz screen:** question card (word + hint), four multiple-choice
  options, "Next Question" / "Check Answer" flow, live score.
- **Round complete:** tally of correct answers, restart button.
- **Settings:** difficulty selector, answer count (4/6/8), sound toggle.
- **Persistence:** high score in localStorage.
- **Assets:** generated procedural "zoo" illustration + word art.

## Tech
- Static HTML/CSS/JS, no build step.
- Vanilla JS for state machine (home -> quiz -> complete).
- localStorage for high score + settings.
- Procedural SVG/CSS art for the zoo theme.

## Phases
1. Scaffold index.html + CSS + JS shell.
2. Data: load the vocabulary sheet JSON, normalize into a question pool.
3. UI: home screen.
4. UI: quiz screen + answer checking.
5. UI: round complete + settings + persistence.
6. Polish: art, transitions, responsive.