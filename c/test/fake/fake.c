/* the fake peripherals of fake/ch643.h */
#include "ch643.h"

GPIO_TypeDef fake_gpioa, fake_gpiob, fake_gpioc;
RCC_TypeDef fake_rcc;
AFIO_TypeDef fake_afio;
EXTI_TypeDef fake_exti;
SysTick_Type fake_systick;
TIM_TypeDef fake_tim2;
LEDPWM_TypeDef fake_ledpwm;
FLASH_TypeDef fake_flash;
uint32_t SystemCoreClock = 48000000;
uint8_t fake_nvic[64];
