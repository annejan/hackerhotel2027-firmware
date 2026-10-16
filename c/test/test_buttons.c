/* buttons.c against fake GPIO: debouncing, press / tap / hold, chords,
 * the ◀ + ▶ boot and A + B bring-up chords, pin mapping. */
#include <string.h>
#include "ch643.h"
#include "buttons.h"
#include "check.h"
#include "events.h"

static char got[4096];
static int n;
void app_event(event_t ev, uint8_t arg) { n += sprintf(got + n, "%c%d ", "PTHCBS"[ev], arg); }

static GPIO_TypeDef *const port[4] = {&fake_gpioc, &fake_gpiob, &fake_gpioc, &fake_gpiob};
static const int pin[4] = {4, 20, 5, 18};
static void set(int i, int down)
{
    if (down)
        port[i]->INDR &= ~(1u << pin[i]);
    else
        port[i]->INDR |= 1u << pin[i];
}
static void polls(int k) { while (k--) buttons_poll(); }
static void expect(const char *what, const char *want)
{
    CHECK(strcmp(got, want) == 0, "%s: got [%s], want [%s]", what, got, want);
    n = 0;
    got[0] = 0;
}

int main(void)
{
    enum { L, B, A, R };
    const int hold = BUTTONS_HOLD_MS / BUTTONS_POLL_MS, chord = BUTTONS_BOOT_MS / BUTTONS_POLL_MS;
    fake_gpiob.INDR = fake_gpioc.INDR = ~0u; /* pull-ups: all up */
    buttons_init();
    CHECK((fake_gpioc.CFGLR >> 16 & 0xFF) == 0x44 && (fake_gpiob.CFGXR & 0x000F0F00u) == 0x00040400u,
          "PC4, PC5, PB18, PB20 floating inputs (CFGLR %08x CFGXR %08x)", fake_gpioc.CFGLR, fake_gpiob.CFGXR);
    polls(5);
    expect("idle", "");

    set(R, 1); polls(1); set(R, 0); polls(5);
    expect("a one-poll glitch is ignored", "");
    set(R, 1); polls(3); set(R, 0); polls(3);
    expect("tap ▶", "P3 T3 ");
    set(L, 1); polls(1); set(L, 0); polls(1); set(L, 1); polls(4); set(L, 0); polls(3);
    expect("bouncy press on ◀ is one tap", "P0 T0 ");

    set(A, 1); polls(hold + 5);
    expect("hold A sends Hold while still down", "P2 H2 ");
    CHECK(buttons_held == 1u << A, "held bit for A (got %02x)", buttons_held);
    set(A, 0); polls(3);
    expect("release after a hold: no tap", "");
    CHECK(buttons_held == 0, "held cleared");

    set(L, 1); set(B, 1); polls(5); set(L, 0); set(B, 0); polls(5);
    expect("chord ◀ + B: presses only, no taps", "P0 P1 ");
    set(L, 1); set(B, 1); polls(hold + 5); set(L, 0); set(B, 0); polls(5);
    expect("chord ◀ + B held: no holds either", "P0 P1 ");

    set(L, 1); set(R, 1); polls(chord + 2);
    expect("◀ + ▶ 3 s: boot ROM, once", "P0 P3 B0 ");
    polls(200);
    expect("no second boot event", "");
    set(L, 0); set(R, 0); polls(3);
    expect("releasing the chord: nothing", "");

    set(A, 1); set(B, 1); polls(chord + 2);
    expect("A + B 3 s: bring-up test, once", "P1 P2 S0 ");
    set(A, 0); set(B, 0); polls(3);
    expect("releasing A + B: nothing", "");

    set(A, 1); polls(50); set(B, 1); polls(chord - 50 + 2);
    expect("A then B: no hold for A (B joined before 0.6 s), no chord yet", "P2 P1 ");
    polls(60);
    expect("the chord counts from the later button", "S0 ");
    set(A, 0); set(B, 0); polls(3);
    expect("released", "");

    for (int i = 0; i < 4; i++) { set(i, 1); polls(3); set(i, 0); polls(3); }
    expect("each button reads its own pin (PC4 PB20 PC5 PB18)", "P0 T0 P1 T1 P2 T2 P3 T3 ");
    TEST_END("buttons");
}
