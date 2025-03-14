.data
.text
.globl f
f:
    addi sp, sp, -64
    sw ra, 60(sp)
    sw s0, 0(sp)
    addi s0, sp, 0
    sw a0, -4(s0)
    lw t0, -4(s0)
    li t1, 0
    xor t0, t0, t1
    seqz t0, t0
    beqz t0, .if_end0
    li t2, 0
    mv a0, t2
    lw ra, 60(sp)
    lw s0, 0(sp)
    addi sp, sp, 64
    ret
.if_end0:
    lw t3, -4(s0)
    lw t4, -4(s0)
    li t5, 1
    sub t4, t4, t5
    mv a0, t4
    call f
    mv t6, a0
    add t3, t3, t6
    mv a0, t3
    lw ra, 60(sp)
    lw s0, 0(sp)
    addi sp, sp, 64
    ret
