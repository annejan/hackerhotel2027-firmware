/* Stand-in for WCH's ch643.h in the host tests: the peripherals are plain
 * structs in RAM (fake.c), with the register and bit names the drivers
 * use. Only what ir.c, buttons.c, ledpwm.c and store.c touch. A test sets
 * inputs (INDR, SysTick CNT) and reads back what the driver wrote. */
#pragma once
#include <stdint.h>

typedef struct { volatile uint32_t CFGLR, CFGHR, INDR, OUTDR, BSHR, BCR, LCKR, CFGXR, BSXR; } GPIO_TypeDef;
typedef struct { volatile uint32_t APB2PCENR, APB1PCENR; } RCC_TypeDef;
typedef struct { volatile uint32_t ECR, PCFR1, EXTICR[2]; } AFIO_TypeDef;
typedef struct { volatile uint32_t INTENR, EVENR, RTENR, FTENR, SWIEVR, INTFR; } EXTI_TypeDef;
typedef struct { volatile uint32_t CTLR, SR; volatile uint64_t CNT, CMP; } SysTick_Type;
typedef struct {
    volatile uint16_t CTLR1, CTLR2, SMCFGR, DMAINTENR, INTFR, SWEVGR, CHCTLR1, CHCTLR2, CCER, CNT, PSC,
        ATRLR, RPTCR, CH1CVR, CH2CVR, CH3CVR, CH4CVR, BDTR;
} TIM_TypeDef;
typedef struct {
    volatile uint8_t COM_SEL, KEY_CTRL, PWM_OE, DMA_CNT, COM_E_PB_L, COM_E_PB_M, COM_E_PB_H, COM_E_PAPC,
        PWM_PIN_L, PWM_PIN_H, RES0, PWM_MOD, FUNC_CTRL, ANA_CTRL, CYCLE_CFG, FRAME_CFG;
    volatile uint16_t DMA;
} LEDPWM_TypeDef;
typedef struct { volatile uint32_t ACTLR, KEYR, OBKEYR, STATR, CTLR, ADDR, RESERVED, OBR, WPR, MODEKEYR, BOOT_MODEKEYR; } FLASH_TypeDef;

extern GPIO_TypeDef fake_gpioa, fake_gpiob, fake_gpioc;
extern RCC_TypeDef fake_rcc;
extern AFIO_TypeDef fake_afio;
extern EXTI_TypeDef fake_exti;
extern SysTick_Type fake_systick;
extern TIM_TypeDef fake_tim2;
extern LEDPWM_TypeDef fake_ledpwm;
extern FLASH_TypeDef fake_flash;
extern uint32_t SystemCoreClock;

#define GPIOA (&fake_gpioa)
#define GPIOB (&fake_gpiob)
#define GPIOC (&fake_gpioc)
#define RCC (&fake_rcc)
#define AFIO (&fake_afio)
#define EXTI (&fake_exti)
#define SysTick (&fake_systick)
#define TIM2 (&fake_tim2)
#define LEDPWM (&fake_ledpwm)
#define FLASH (&fake_flash)

#define RCC_AFIOEN 0x1
#define RCC_IOPAEN 0x4
#define RCC_IOPBEN 0x8
#define RCC_IOPCEN 0x10
#define RCC_TIM2EN 0x1
#define AFIO_PCFR1_TIM2_REMAP 0x1C0000
#define AFIO_PCFR1_TIM2_REMAP_0 0x40000
#define TIM_CEN 0x1
#define TIM_UG 0x1
#define TIM_ARPE 0x80
#define TIM_OC1PE 0x0008
#define TIM_CC1E 0x0001
#define TIM_CC1P 0x0002
#define TIM_MOE 0x8000

enum { SysTicK_IRQn = 12, LEDPWM_IRQn = 19, EXTI25_16_IRQn = 41 };
/* NVIC_EnableIRQ(n) sets fake_nvic[n] */
extern uint8_t fake_nvic[64];
static inline void NVIC_EnableIRQ(int n) { fake_nvic[n] = 1; }
