; Array summation — sum all elements of arr[0..N-1].
; Result: R3 = sum = 10+20+30+40+50 = 150

.program "sum_array"
.version 1
.memory 64
.data
arr: .words 10, 20, 30, 40, 50

.text
.entry main

main:
    IMML R1, 0          ; base address of arr (data loads at addr 0)
    IMML R2, 5          ; N = 5 (array length)
    IMML R3, 0          ; sum = 0
    IMML R4, 0          ; i = 0

loop:
    CMP  R4, R2
    BGE  done           ; exit when i >= N

    ADD  R5, R1, R4     ; addr = base + i
    LOAD R5, R5         ; R5 = arr[i]
    ADD  R3, R3, R5     ; sum += arr[i]
    INC  R4, R4         ; i++
    JMP  loop

done:
    ; R3 = 150
    HALT
