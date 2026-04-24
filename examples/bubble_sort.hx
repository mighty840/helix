; Bubble sort — sort arr[0..N-1] in ascending order.
; Input:  arr = [5, 3, 8, 1, 9, 2, 7, 4]
; Output: arr = [1, 2, 3, 4, 5, 7, 8, 9]
;
; Note: IMML only sets the low byte of a register.
; For this program all values fit in 8 bits, so high bytes stay 0.

.program "bubble_sort"
.version 1
.memory 128
.data
arr: .words 5, 3, 8, 1, 9, 2, 7, 4

.text
.entry main

; Registers:
;   R1 = base address (0)
;   R2 = N (8)
;   R3 = outer loop counter i
;   R4 = inner loop counter j
;   R5 = arr[j]        (loaded for comparison / swap)
;   R6 = arr[j+1]      (loaded for comparison / swap)
;   R7 = scratch / address temp

main:
    IMML R1, 0
    IMML R2, 8
    IMML R3, 0          ; i = 0

outer_check:
    IMML R7, 1
    SUB  R7, R2, R7     ; R7 = N - 1
    CMP  R3, R7
    BGE  done           ; if i >= N-1, done

    IMML R4, 0          ; j = 0

inner_check:
    IMML R7, 1
    SUB  R7, R2, R7     ; R7 = N - 1
    SUB  R7, R7, R3     ; R7 = N - 1 - i  (inner limit)
    CMP  R4, R7
    BGE  outer_next     ; if j >= limit, next outer pass

    ; Load arr[j] and arr[j+1], save their addresses on the stack.
    ADD  R7, R1, R4     ; R7 = addr_j = base + j
    LOAD R5, R7         ; R5 = arr[j]
    PUSH R7             ; save addr_j

    IMML R7, 1
    ADD  R7, R7, R4     ; R7 = 1 + j
    ADD  R7, R7, R1     ; R7 = addr_j1 = base + j + 1
    LOAD R6, R7         ; R6 = arr[j+1]
    PUSH R7             ; save addr_j1

    ; If arr[j] <= arr[j+1] then no swap needed.
    CMP  R5, R6
    BLT  no_swap        ; arr[j] < arr[j+1]: already ordered
    BEQ  no_swap        ; arr[j] == arr[j+1]: no change needed

    ; Swap: write arr[j] → addr_j1, arr[j+1] → addr_j.
    POP  R7
    STORE R7, R5        ; arr[j+1] = old arr[j]
    POP  R7
    STORE R7, R6        ; arr[j] = old arr[j+1]
    JMP  inner_next

no_swap:
    POP  R7             ; discard addr_j1
    POP  R7             ; discard addr_j

inner_next:
    INC  R4, R4         ; j++
    JMP  inner_check

outer_next:
    INC  R3, R3         ; i++
    JMP  outer_check

done:
    HALT
