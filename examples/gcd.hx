; Euclidean GCD — compute GCD(a, b).
; Result: R1 = GCD(48, 18) = 6
;
; Algorithm: while b != 0: (a, b) = (b, a mod b)

.program "gcd"
.version 1
.text
.entry main

main:
    IMML R1, 48         ; a = 48
    IMML R2, 18         ; b = 18

loop:
    CMP  R2, R0         ; b == 0?
    BEQ  done           ; yes → done, R1 holds GCD

    MOD  R3, R1, R2     ; R3 = a mod b
    ADD  R1, R2, R0     ; a = b
    ADD  R2, R3, R0     ; b = a mod b
    JMP  loop

done:
    ; R1 = GCD(48, 18) = 6
    HALT
