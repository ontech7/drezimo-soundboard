//! Turns screens into the frames the display shows, with what moves: long names scrolling by, what blinks, a new
//! page sliding in. A frame goes out only when its pixels change.

use super::screens::{self, FRAME, Motion, Screen};
use super::{Canvas, Frame, WIDTH};
use std::time::{Duration, Instant};

const SLIDE_TIME: Duration = Duration::from_millis(200);
/// What slides: all but the voice and the switches at the bottom
const SLIDING_ROWS: usize = 48;

struct Slide {
    from: Canvas,
    started: Instant,
    forward: bool,
}

#[derive(Default)]
pub struct Animator {
    /// The big name shown, and since when: a long one scrolls from its start
    name: Option<(String, Instant)>,
    /// What blinks or lights up in turn, and since when: it starts lit
    rhythm: Option<(usize, Instant)>,
    /// A page turned since the last frame, forward or not
    turned: Option<bool>,
    slide: Option<Slide>,
    shown: Option<Canvas>,
    next: Option<Instant>,
}

impl Animator {
    /// The frame for `screen` at `now`, unless the display shows it already.
    pub fn frame(&mut self, screen: &Screen, now: Instant) -> Option<Frame> {
        if self.name.as_ref().map(|(name, _)| name.as_str()) != screen.name() {
            self.name = screen.name().map(|name| (name.to_string(), now));
        }
        if self.rhythm.map(|(rhythm, _)| rhythm) != screen.rhythm() {
            self.rhythm = screen.rhythm().map(|rhythm| (rhythm, now));
        }
        let since = |start: Option<Instant>| start.map_or(Duration::ZERO, |start| now.saturating_duration_since(start));
        let motion = Motion {
            name_for: since(self.name.as_ref().map(|(_, start)| *start)),
            clock: since(self.rhythm.map(|(_, start)| start)),
        };
        let (mut canvas, changes) = screens::draw(screen, motion);

        // From what the display shows, even halfway through another slide
        if let Some(forward) = self.turned.take()
            && let Some(from) = &self.shown
        {
            self.slide = Some(Slide {
                from: from.clone(),
                started: now,
                forward,
            });
        }
        if !matches!(screen, Screen::Home { .. }) {
            self.slide = None;
        }

        self.next = changes.map(|after| now + after);
        if let Some(slide) = &self.slide {
            let done = now.saturating_duration_since(slide.started).as_secs_f32() / SLIDE_TIME.as_secs_f32();
            if done < 1.0 {
                // Gently in and out
                let eased = done * done * (3.0 - 2.0 * done);
                canvas.slide_from(
                    &slide.from,
                    (WIDTH as f32 * eased).round() as i32,
                    slide.forward,
                    SLIDING_ROWS,
                );
                self.next = Some(self.next.map_or(now + FRAME, |next| next.min(now + FRAME)));
            } else {
                self.slide = None;
            }
        }

        if self.shown.as_ref() == Some(&canvas) {
            return None;
        }
        let frame = canvas.frame();
        self.shown = Some(canvas);
        Some(frame)
    }

    /// The next frame slides the new page in: from the right going forward.
    pub fn page_turned(&mut self, forward: bool) {
        self.turned = Some(forward);
    }

    /// When the picture changes by itself.
    pub fn next_frame(&self) -> Option<Instant> {
        self.next
    }

    /// For a display whose picture is unknown, as one just connected: the next frame goes out whatever it is.
    pub fn forget(&mut self) {
        self.shown = None;
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::margin_is_dark;
    use super::*;
    use std::ops::Range;

    /// Two frames into a slide, the pictures have moved this far
    const OFFSET_TWO_FRAMES_IN: i32 = 45;

    fn home(title: &str, name: &str) -> Screen {
        Screen::Home {
            title: title.into(),
            counter: "1/5".into(),
            name: name.into(),
            progress: None,
            voice: Some("Robot".into()),
            voice_on: true,
            switches: Vec::new(),
        }
    }

    fn first_lit(canvas: &Canvas, columns: Range<i32>, rows: Range<i32>) -> (i32, i32) {
        columns
            .flat_map(|x| rows.clone().map(move |y| (x, y)))
            .find(|(x, y)| canvas.is_lit(*x, *y))
            .expect("a lit pixel")
    }

    /// What the display shows two frames into a slide from `from` to `to`
    fn halfway(from: &Screen, to: &Screen, forward: bool) -> Canvas {
        let now = Instant::now();
        let mut animator = Animator::default();
        animator.frame(from, now);
        animator.page_turned(forward);
        assert!(animator.frame(to, now).is_none(), "the old page, still");
        assert_eq!(animator.next_frame(), Some(now + FRAME));
        animator.frame(to, now + FRAME * 2).expect("a frame");
        let shown = animator.shown.clone().unwrap();
        let (end, _) = screens::draw(to, Motion::default());
        assert_eq!(animator.frame(to, now + SLIDE_TIME), Some(end.frame()));
        assert_eq!(animator.next_frame(), None);
        shown
    }

    #[test]
    fn sends_a_frame_only_when_the_picture_changes() {
        let now = Instant::now();
        let mut animator = Animator::default();
        assert!(animator.frame(&home("A", "Ready"), now).is_some());
        assert!(animator.frame(&home("A", "Ready"), now + FRAME).is_none());
        assert_eq!(animator.next_frame(), None);
        animator.forget();
        assert!(animator.frame(&home("A", "Ready"), now + FRAME).is_some());
    }

    #[test]
    fn slides_only_on_a_page_turn() {
        let now = Instant::now();
        let mut animator = Animator::default();
        animator.frame(&home("A", "Ready"), now);
        animator.frame(&home("B", "Ready"), now);
        assert_eq!(animator.next_frame(), None);
    }

    #[test]
    fn going_forward_the_new_page_comes_in_from_the_right() {
        let to = home("B", "Ready");
        let shown = halfway(&home("A", "Ready"), &to, true);
        let (end, _) = screens::draw(&to, Motion::default());
        let (x, y) = first_lit(&end, 0..WIDTH, 2..9);
        assert!(shown.is_lit(x + WIDTH - OFFSET_TWO_FRAMES_IN, y));
        assert_eq!(
            first_lit(&shown, 0..WIDTH, 53..62),
            first_lit(&end, 0..WIDTH, 53..62),
            "the strip stays"
        );
        assert!(margin_is_dark(&shown));
    }

    #[test]
    fn going_back_the_new_page_comes_in_from_the_left() {
        let to = home("B", "Ready");
        let shown = halfway(&home("A", "Ready"), &to, false);
        let (end, _) = screens::draw(&to, Motion::default());
        // The counter, on the right of the new page, shows first
        let (x, y) = first_lit(&end, WIDTH - OFFSET_TWO_FRAMES_IN..WIDTH, 2..9);
        assert!(shown.is_lit(x - (WIDTH - OFFSET_TWO_FRAMES_IN), y));
        assert!(margin_is_dark(&shown));
    }

    #[test]
    fn scrolls_a_new_name_from_its_start() {
        let now = Instant::now();
        let mut animator = Animator::default();
        let long = home("A", "Applause from the stadium");
        animator.frame(&long, now);
        animator.frame(&long, now + Duration::from_secs(2));
        animator.frame(&home("A", "Honk"), now + Duration::from_secs(3));
        let again = animator.frame(&long, now + Duration::from_secs(4)).unwrap();
        assert_eq!(again, screens::draw(&long, Motion::default()).0.frame());
    }
}
