.text
.globl add
add:
    addi sp, sp, -32
    sw ra, 28(sp)
    sw s0, 24(sp)
    addi s0, sp, 32
    addi sp, sp, -32
    sw a0, -36(s0)
    sw a1, -40(s0)
    sw a2, -44(s0)
    sw a3, -48(s0)
    sw a4, -52(s0)
    sw a5, -56(s0)
    sw a6, -60(s0)
    sw a7, -64(s0)
    lw t0, 0(fp)
    sw t0, -68(s0)
    lw t0, 4(fp)
    sw t0, -72(s0)
    lw t0, 8(fp)
    sw t0, -76(s0)
    lw t0, 12(fp)
    sw t0, -80(s0)
    lw t0, 16(fp)
    sw t0, -84(s0)
    lw t0, 20(fp)
    sw t0, -88(s0)
    lw t0, 24(fp)
    sw t0, -92(s0)
    lw t0, 28(fp)
    sw t0, -96(s0)
    lw t0, 32(fp)
    sw t0, -100(s0)
    lw t0, 36(fp)
    sw t0, -104(s0)
    lw t0, 40(fp)
    sw t0, -108(s0)
    lw t0, 44(fp)
    sw t0, -112(s0)
    lw t0, 48(fp)
    sw t0, -116(s0)
    lw t0, 52(fp)
    sw t0, -120(s0)
    lw t0, 56(fp)
    sw t0, -124(s0)
    lw t0, 60(fp)
    sw t0, -128(s0)
    lw t0, 64(fp)
    sw t0, -132(s0)
    lw t0, 68(fp)
    sw t0, -136(s0)
    lw t0, -136(s0)
    lw t1, -132(s0)
    add t0, t0, t1
    lw t1, -128(s0)
    add t0, t0, t1
    lw t1, -124(s0)
    add t0, t0, t1
    lw t1, -120(s0)
    add t0, t0, t1
    lw t1, -116(s0)
    add t0, t0, t1
    lw t1, -112(s0)
    add t0, t0, t1
    lw t1, -108(s0)
    add t0, t0, t1
    lw t1, -104(s0)
    add t0, t0, t1
    lw t1, -100(s0)
    add t0, t0, t1
    lw t1, -96(s0)
    add t0, t0, t1
    lw t1, -92(s0)
    add t0, t0, t1
    lw t1, -88(s0)
    add t0, t0, t1
    lw t1, -84(s0)
    add t0, t0, t1
    lw t1, -80(s0)
    add t0, t0, t1
    lw t1, -76(s0)
    add t0, t0, t1
    lw t1, -72(s0)
    add t0, t0, t1
    lw t1, -68(s0)
    add t0, t0, t1
    lw t1, -64(s0)
    add t0, t0, t1
    lw t1, -60(s0)
    add t0, t0, t1
    lw t1, -56(s0)
    add t0, t0, t1
    lw t1, -52(s0)
    add t0, t0, t1
    lw t1, -48(s0)
    add t0, t0, t1
    lw t1, -36(s0)
    add t0, t0, t1
    lw t1, -40(s0)
    add t0, t0, t1
    lw t1, -44(s0)
    add t0, t0, t1
    mv a0, t0
    mv sp, s0
    addi sp, sp, -32
    lw s0, 24(sp)
    lw ra, 28(sp)
    addi sp, sp, 32
    ret
.globl main
main:
    addi sp, sp, -32
    sw ra, 28(sp)
    sw s0, 24(sp)
    addi s0, sp, 32
    addi sp, sp, -32
    addi sp, sp, -0
    addi sp, sp, -88
    li t0, 1
    mv a0, t0
    li t0, 2
    mv a1, t0
    li t0, 3
    mv a2, t0
    li t0, 4
    mv a3, t0
    li t0, 5
    mv a4, t0
    li t0, 6
    mv a5, t0
    li t0, 7
    mv a6, t0
    li t0, 8
    mv a7, t0
    li t0, 9
    sw t0, 0(sp)
    li t0, 10
    sw t0, 4(sp)
    li t0, 11
    sw t0, 8(sp)
    li t0, 12
    sw t0, 12(sp)
    li t0, 13
    sw t0, 16(sp)
    li t0, 14
    sw t0, 20(sp)
    li t0, 15
    sw t0, 24(sp)
    li t0, 16
    sw t0, 28(sp)
    li t0, 17
    sw t0, 32(sp)
    li t0, 18
    sw t0, 36(sp)
    li t0, 19
    sw t0, 40(sp)
    li t0, 20
    sw t0, 44(sp)
    li t0, 21
    sw t0, 48(sp)
    li t0, 22
    sw t0, 52(sp)
    li t0, 23
    sw t0, 56(sp)
    li t0, 24
    sw t0, 60(sp)
    li t0, 25
    sw t0, 64(sp)
    li t0, 26
    sw t0, 68(sp)
    li t0, 27
    sw t0, 72(sp)
    li t0, 28
    sw t0, 76(sp)
    li t0, 29
    sw t0, 80(sp)
    li t0, 30
    sw t0, 84(sp)
    call add
    addi sp, sp, 88
    addi sp, sp, 0
    mv t0, a0
    li t1, 30
    xor t0, t0, t1
    seqz t0, t0
    seqz t0, t0
    mv a0, t0
    mv sp, s0
    addi sp, sp, -32
    lw s0, 24(sp)
    lw ra, 28(sp)
    addi sp, sp, 32
    ret
