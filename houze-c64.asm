; A BASIC booter, encodes `10 SYS <address>`.
; Macroified from http://www.pouet.net/topic.php?which=6541

	basic = $0801
	bocol = $d020
	bgcol = $d021

	; KERNAL methods
	SETLFS = $FFBA
	OPEN = $FFC0
	SETNAM = $FFBD
	READST = $FFB7
	CHKOUT = $FFC9
	CHROUT = $FFD2
	CHKIN = $FFC6
	CHRIN = $FFCF
	PLOT = $FFF0
	GETIN = $FFE4
	SCNKEY = $FF9F
	SETMSG = $FF90
	CLOSE = $FFC3
	PLOT = $FFF0
	SCINIT = $FF81
	CLRCHN = $FFCC
	RSSTAT = $0297
	RDTIM = $FFDE

	; rs232 buffer pointers
	RS232_INBUF_PTR = $f7
	RS232_OUTBUF_PTR = $f9

	!macro start_at .address {
		* = basic
		!byte $0c,$08,$00,$00,$9e
		!if .address >= 10000 { !byte 48 + ((.address / 10000) % 10) }
		!if .address >=	 1000 { !byte 48 + ((.address /	 1000) % 10) }
		!if .address >=		100 { !byte 48 + ((.address /		100) % 10) }
		!if .address >=		 10 { !byte 48 + ((.address /		 10) % 10) }
		!byte $30 + (.address % 10), $00, $00, $00
		* = .address
	}

	roomsize1 = $08df
	roomitems = $08e0
	roomsize2 = $08e1
	roombgcol = $08e2
	
	roomdirty = $08ef
	
	screenpos = $08f0

	!source "macros.asm"

+start_at $0900

.init
	;; ldx #23
	;; stx 53272

	;; disable BASIC rom
	;; lda $01
	;; and #%11111110
	;; sta $01
	
	ldx #$00
	stx bocol
	stx bgcol
	jsr rs232_open
	lda #$01
	sta roomdirty
	
	lda #5
	sta roomsize1
	lda #34
	sta roomsize2

.main_loop
	lda roomdirty
	cmp #$01
	bne .skip_load_room
	lda #$20
	jsr .clear_screen
	jsr .load_and_paint_room
.skip_load_room
	jsr .keyboard_read
	jmp .main_loop
	
.keyboard_read
	jsr SCNKEY
	jsr GETIN
	cmp #0
	beq .kbd_waitup
	cmp #'N'
	beq .go_north
	cmp #'S'
	beq .go_south
	cmp #'W'
	beq .go_west
	cmp #'E'
	beq .go_east
	cmp #'T'
	beq .take_item
.kbd_waitup
	;; jsr SCNKEY
	jsr GETIN
	cmp #0
	bne .kbd_waitup
	rts

;; TODO item index
.take_item
	+set16im .cmd_item_take, $fb
	jmp .go_send
.go_north
	+set16im .cmd_go_north, $fb
	jmp .go_send
.go_south
	+set16im .cmd_go_south, $fb
	jmp .go_send
.go_west
	+set16im .cmd_go_west, $fb
	jmp .go_send
.go_east
	+set16im .cmd_go_east, $fb
.go_send
	jsr rs232_send_string
	+set16im $0400, $fb
	lda #0
	sta screenpos
	jsr .recv_and_draw
	
	lda #$01
	sta roomdirty

	;; FIXME
	jmp .kbd_waitup

.load_and_paint_room
	jsr .load_room_title
	jsr .load_room_appearance
	jsr .load_room_title
	jsr .load_room_desc

	lda #$20
	jsr .clear_room
	jsr .paint_room
	lda #$00
	sta roomdirty

	;; TODO load room items
	jsr .load_item_titles
	
	rts
	
;;; graphics

.clear_screen
	ldx #$00
.cls_loop
	sta $0400,x	
	sta $0500,x
	sta $0600,x
	sta $0700,x
	dex
	bne .cls_loop
	rts
	
.clear_room
	ldx #$00
.cls_loop2	
	sta $0500,x
	sta $0600,x
	sta $0700,x
	dex
	bne .cls_loop2
	rts
	
.recv_and_draw
	jsr rs232_try_read_byte
	;; end of line?
	cmp #10
	beq .recv_and_draw_done
	cmp #0
	bne .got_byte
	jmp .recv_and_draw
.got_byte
	;; inc bocol
	ldy screenpos
	and #$3f
	sta ($fb), y
	iny
	sty screenpos
	jmp .recv_and_draw
.recv_and_draw_done
	rts

.load_room_title
	+set16im .cmd_room_title, $fb
	jsr rs232_send_string
	lda #0
	sta screenpos
	+set16im $0400, $fb
	jsr .recv_and_draw
	rts
	
.load_room_appearance
	+set16im .cmd_room_appearance, $fb
	jsr rs232_send_string
	lda #0
	sta screenpos
	;; hack: load directly into room variables
	+set16im roomitems, $fb
	jsr .recv_and_draw
	
	sec
	lda roomitems
	sbc #48
	sta roomitems
	
	sec
	lda roomsize2
	sbc #48
	asl
	adc #16
	sta roomsize2
	
	sec
	lda roombgcol
	sbc #48
	sta roombgcol
	sta bgcol
	sta bocol
		
	rts
	
.load_room_desc
	+set16im .cmd_room_desc, $fb
	jsr rs232_send_string
	lda #0
	sta screenpos
	+set16im $0450, $fb
	jsr .recv_and_draw
	rts

.itemnum	!byte 0
	
.load_item_titles
	lda roomitems
	bne .item_start
	rts
.item_start
	sta .itemnum
.item_loop
	dec .itemnum
	+set16im .cmd_item_title, $fb
	lda .itemnum
	clc
	adc #48
	ldy #2
	sta ($fb), y									; item index number
	jsr rs232_send_string

	lda #0
	sta screenpos
	+set16im $05a4, $fb
	ldy .itemnum
	iny
.item_lineloop
	+add16im $fb, 40, $fb
	dey
	bne .item_lineloop
	
	jsr .recv_and_draw
	
	lda .itemnum
	bne .item_loop								; more items to load?
	rts

.cmd_room_title !text "TR",10,0
.cmd_room_appearance !text "AR",10,0
.cmd_room_desc	!text "DR",10,0
.cmd_item_title !text "TI0",10,0
.cmd_item_take	!text "PI0",10,0
.cmd_go_north		!text "GN",10,0
.cmd_go_south		!text "GS",10,0
.cmd_go_west		!text "GW",10,0
.cmd_go_east		!text "GE",10,0
	
.paint_room
	+set16im $0540, $fb
	ldx #10
.pr_loop
	lda #$6a				; |
	ldy roomsize1
	sta ($fb), y
	lda #$74				; |
	ldy roomsize2
	sta ($fb), y

	;; we could fill the wall with a pattern here
	
	+add16im $fb, 40, $fb
	dex
	bne .pr_loop

	ldy roomsize2
	dey
	lda #$77				; _
.pr_loop3
	sta ($fb), y
	dey
	cpy #5					; FIXME
	bne .pr_loop3

	ldy roomsize1
	ldx roomsize2
.pr_loop2
	lda #$4e				; /
	sta ($fb), y
	tya
	pha
	txa
	tay
	lda #$4d				; \
	sta ($fb), y
	pla
	tay
	+add16im $fb, 40, $fb
	inx
	dey
	bne .pr_loop2
	rts

	!source "rs232.asm"
