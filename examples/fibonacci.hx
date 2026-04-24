; Fibonacci sequence — compute F(N) iteratively.
; Result: R1 = F(10) = 55
;
; Algorithm: starting from a=F(0)=0, b=F(1)=1,
; iterate N times: (a, b) = (b, a+b)
; After N iterations, a = F(N).

.program "fibonacci"
.version 1
.text
.entry main

main:
    IMML R1, 0          ; a = F(0) = 0
    IMML R2, 1          ; b = F(1) = 1
    IMML R3, 0          ; counter = 0
    IMML R4, 10         ; N = 10

loop:
    CMP  R3, R4
    BGE  done           ; exit when counter >= N

    ADD  R5, R1, R2     ; temp = a + b
    ADD  R1, R2, R0     ; a = b  (ADD with R0 = move)
    ADD  R2, R5, R0     ; b = temp
    INC  R3, R3         ; counter++
    JMP  loop

done:
    ; R1 = F(10) = 55
    HALT
