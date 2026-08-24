#!/usr/bin/env bash
#this is 100% ai generated. macos probably can't run it

#the borger dev status bar, sourced by cli.sh. it only reads from its caller, never the
#other way round: SERVER_BUILD_ARGS and SERVER_CMD are what it pgreps for. cli.sh calls
#sb_is_supported, sb_open, sb_loop and sb_close, and nothing else in here is anyone
#else's business.
#
#the server is watched through the process table, since a running binary is the only
#honest answer to whether it is up. the client is watched through its watcher's own
#log lines, because that is one clock rather than two: its wasm hits the disk a beat
#before the line saying so, and anything reading both sees states that never coexisted

#every scrap of log text the bar keys off, in one place because none of it is ours to
#choose: the first three are cargo watch's wording, Local is vite's banner, and the tags
#are the pane names cli.sh hands to concurrently. if one of them stops matching, this is
#the list to check against whatever the tool prints now
SB_LOG_RUNNING="[Running "
SB_LOG_COMPILE_FAIL="could not compile"
SB_LOG_FINISHED="Finished running"
SB_LOG_LOCAL="Local"
SB_TAG_SERVER_RUST="SERVER-RUST"
SB_TAG_CLIENT_RUST="CLIENT-RUST"
SB_TAG_CLIENT_VITE="CLIENT-VITE"

#the same again for the lines read rather than merely spotted: the code cargo watch signs
#a run off with, the count the typescript checker prints, the url buried in vite's banner,
#and the colour escapes wrapped around it
SB_RE_EXIT_STATUS='[Ee]xit status:?[[:space:]]*([0-9]+)'
SB_RE_TS_ERRORS='Found ([0-9]+) error'
SB_RE_URL='(https?://[^[:space:]]+)'
SB_RE_ANSI=$'\e''\[[0-9;]*[a-zA-Z]'

SB_TICK=0.25
SB_COLS=80
SB_ON=0
SB_SHOWN=0
SB_LAST=-1
SB_SEEN=0
SB_SERVER_FAILED=0
SB_CLIENT_STATE="building"
SB_TS_FAILED=0
SB_TS_GRACE=0
SB_PHASE=""
SB_BUILD_AT=0
SB_BUILD_FOR=""
SB_BAR=""
SB_PAINT=""
SB_URL=""

#bakes one state's pair of #rrggbb knobs into the escape that paints it. 38 puts the
#colour on the glyphs and 48 on the cell behind them, and the ;2; is what says the next
#three values are a plain 24 bit rgb triple rather than a palette index
sb_sgr()
{
	local text="${2#\#}" back="${3#\#}"

	printf -v "$1" '\e[48;2;%d;%d;%dm\e[38;2;%d;%d;%dm' \
		"0x${back:0:2}" "0x${back:2:2}" "0x${back:4:2}" \
		"0x${text:0:2}" "0x${text:2:2}" "0x${text:4:2}"
}

SB_DOWN_TEXT="#000000"
SB_DOWN_BG="#ff66f2"
SB_UP_TEXT="#000000"
SB_UP_BG="#00ff00"
SB_BAD_TEXT="#000000"
SB_BAD_BG="#ff0000"
sb_sgr SB_DOWN_SGR "$SB_DOWN_TEXT" "$SB_DOWN_BG"
sb_sgr SB_UP_SGR   "$SB_UP_TEXT"   "$SB_UP_BG"
sb_sgr SB_BAD_SGR  "$SB_BAD_TEXT"  "$SB_BAD_BG"

sb_size()
{
	local size
	size=$({ stty size </dev/tty; } 2>/dev/null) || size=""
	if [[ "$size" =~ ^[0-9]+[[:space:]]+([0-9]+)$ ]]; then
		SB_COLS="${BASH_REMATCH[1]}"
	fi
}

sb_is_supported()
{
	[ -t 1 ] && [ -r /dev/tty ]
}

sb_is_server_building()
{
	local by_name by_args pid
	
	#a process actually named cargo that is building the server profile
	by_name=$(pgrep -x cargo 2>/dev/null) || return 1
	by_args=$(pgrep -f -- "$SERVER_BUILD_ARGS" 2>/dev/null) || return 1
	by_args=" ${by_args//$'\n'/ } "

	for pid in $by_name; do
		case "$by_args" in
			*" $pid "*) return 0 ;;
		esac
	done

	return 1
}

#the bar is nothing more than the last line of output: wiped before anything else is
#printed and drawn again underneath it, the way a progress bar works. it deliberately
#isn't pinned to the bottom row, because pinning means fencing that row off with a
#scroll region, and a resize resets the region and reflows the cursor out from under
#it. apt does the same thing and comes apart the same way when you drag the window.
#
#\r\e[J is the wipe: back to the first column, then erase this row and everything under
#it. erasing below matters as well, since narrowing the window reflows a full width bar
#onto two rows and the second one would otherwise be left stranded
sb_erase()
{
	if (( SB_SHOWN == 0 )); then
		return 0
	fi

	SB_SHOWN=0
	printf '\r\e[J'
}

sb_paint()
{
	if (( SB_ON == 0 )); then
		return 0
	fi

	SB_SHOWN=1
	printf '\e[?2026h%s\e[?2026l' "$SB_PAINT"
}

sb_open()
{
	SB_ON=1
	sb_compose
	sb_paint
}

#safe to call twice: the pump drops the bar the moment ctrl+c lands, and the main shell
#drops it again on the way out no matter which way it got there
sb_close()
{
	if (( SB_ON == 0 )); then
		return 0
	fi

	SB_ON=0
	sb_erase
	printf '\e[?25h'
}

#cargo watch announces every run it starts and the status it ended with, and that is the
#only place a compile error turns into a plain yes or no. the process table can't help
#here: a build that failed and a build that was never started look identical from there.
#$1 is the flag to write, $2 the line
sb_build_result()
{
	case "$2" in
		*"$SB_LOG_RUNNING"*)      printf -v "$1" 0 ;;
		*"$SB_LOG_COMPILE_FAIL"*) printf -v "$1" 1 ;;
		*"$SB_LOG_FINISHED"*)
			if [[ "$2" =~ $SB_RE_EXIT_STATUS ]] && (( BASH_REMATCH[1] != 0 )); then
				printf -v "$1" 1
			fi
			;;
	esac
}

#the two things the process table can't tell us: which port vite landed on, and whether
#a build came back with an error. concurrently's name tags say which pane a line is from
sb_scan()
{
	case "$1" in
		*"$SB_TAG_SERVER_RUST"*)
			sb_build_result SB_SERVER_FAILED "$1"
			;;
		*"$SB_TAG_CLIENT_RUST"*)
			#the whole client rust state comes from what its watcher says. watching for
			#the wasm to appear on disk instead would be a second clock: the file lands
			#a beat before the line does, and anything sampling in between sees a wasm
			#that is ready with a typescript verdict that predates it, which reads as
			#failed. one source of truth, no gap.
			#
			#a clean exit also opens grace, since the wasm just changed under vite's feet
			#and its last word was about a tree without it. a new run starting closes
			#grace again, the wasm is on its way out
			case "$1" in
				*"$SB_LOG_RUNNING"*)
					SB_CLIENT_STATE="building"
					SB_TS_GRACE=0
					;;
				*"$SB_LOG_COMPILE_FAIL"*)
					SB_CLIENT_STATE="failed"
					;;
				*"$SB_LOG_FINISHED"*)
					if [[ "$1" =~ $SB_RE_EXIT_STATUS ]] && (( BASH_REMATCH[1] != 0 )); then
						SB_CLIENT_STATE="failed"
					else
						SB_CLIENT_STATE="ready"
						SB_TS_GRACE=1
					fi
					;;
			esac
			;;
		*"$SB_TAG_CLIENT_VITE"*)
			#a fresh count means the checker has looked again, so it gets its say back
			if [[ "$1" =~ $SB_RE_TS_ERRORS ]]; then
				SB_TS_FAILED=$(( BASH_REMATCH[1] != 0 ))
				SB_TS_GRACE=0
			fi
			;;
	esac

	#the url vite prints has color escapes buried inside it, hence the strip
	case "$1" in
		*"$SB_LOG_LOCAL"*) ;;
		*) return 0 ;;
	esac

	local clean="$1"
	while [[ "$clean" =~ $SB_RE_ANSI ]]; do
		clean="${clean//"${BASH_REMATCH[0]}"/}"
	done

	if [[ "$clean" =~ $SB_RE_URL ]]; then
		SB_URL="${BASH_REMATCH[1]}"
	fi
}

#the current time in hundredths of a second, written into the variable named by $1.
#EPOCHREALTIME is a bash 5 builtin and costs nothing to read; older bash only counts
#whole seconds, so there the last two digits are always zero rather than wrong
sb_now()
{
	local raw="${EPOCHREALTIME:-}"

	if [ -n "$raw" ]; then
		#strip the decimal separator, whichever character the locale uses, leaving
		#microseconds, then drop the last four digits to land on hundredths
		raw="${raw//[^0-9]/}"
		printf -v "$1" '%s' "${raw:0:${#raw} - 4}"
	else
		printf -v "$1" '%s' "$(( SECONDS * 100 ))"
	fi
}

#hundredths of a second into something worth reading at a glance
sb_duration()
{
	local secs=$(( $2 / 100 )) rest=$(( $2 % 100 ))

	if (( secs >= 60 )); then
		printf -v "$1" '%dm %02d.%02ds' "$(( secs / 60 ))" "$(( secs % 60 ))" "$rest"
	else
		printf -v "$1" '%d.%02ds' "$secs" "$rest"
	fi
}

#everything the bar knows that did not come from looking at the process table, rolled
#into one string. the pump diffs it either side of a scan so a log line that actually
#moved something repaints straight away instead of waiting out the poll
sb_state_id()
{
	printf -v "$1" '%s|%s|%s|%s|%s' \
		"$SB_SERVER_FAILED" "$SB_CLIENT_STATE" "$SB_TS_FAILED" "$SB_TS_GRACE" "$SB_URL"
}

#work out what the bar should say and bake it, colors and padding and all, into one
#string. this is the expensive half, since it shells out to stty and pgrep, so the pump
#only runs it on a poll or a state change while the paint above runs after every line
sb_compose()
{
	local text sgr phase server client now url="" stamp=""

	#a live process means it compiled, launched, and isn't in the restart nap. cargo
	#still running means a build is underway. calling it crashed takes more than the
	#absence of both, though: the handoff from cargo finishing to the binary coming up
	#is its own little gap, and sampling that gap is what made the bar flash crashed
	#for a tick on startup. so only a server this run has actually seen alive can crash,
	#and a build that came back with an error is its own thing again
	if pgrep -f -x "$SERVER_CMD" >/dev/null 2>&1; then
		SB_SEEN=1
		server="ready"
	elif sb_is_server_building; then
		SB_SEEN=0
		server="building"
	elif (( SB_SERVER_FAILED )); then
		server="failed"
	elif (( SB_SEEN )); then
		server="crashed"
	else
		server="building"
	fi

	#the client is two builds in a trenchcoat, wasm and typescript, and it is only ready
	#once both are. building outranks failed on purpose: while the wasm is rebuilding its
	#pkg folder is gone and typescript loudly cannot find it until it comes back, which
	#is noise from a build in flight rather than a broken one. grace counts as building
	#for the same reason, the wasm has landed but vite has not looked at it yet
	if [[ "$SB_CLIENT_STATE" == "building" ]] || (( SB_TS_GRACE )); then
		client="building"
	elif [[ "$SB_CLIENT_STATE" == "failed" ]] || (( SB_TS_FAILED )); then
		client="failed"
	else
		client="ready"
	fi

	#three phases, keyed off the states about to be printed rather than the raw flags so
	#the bar can't contradict itself. a failed build isn't loading, nothing will happen
	#until the code is fixed, so it gets its own colour rather than a hopeful pink that
	#never resolves. a crashed server is red for the same reason and one more besides:
	#it keeps a server bouncing on its restart nap from reading as a rebuild
	if [[ "$server" == "failed" || "$client" == "failed" || "$server" == "crashed" ]]; then
		phase="bad"
	elif [[ "$server" == "ready" && "$client" == "ready" ]]; then
		phase="up"
	else
		phase="down"
	fi

	#the clock only runs while something is genuinely being built. coming up out of red
	#means whatever just happened wasn't a build, so there is no time worth quoting
	case "$phase" in
		down)
			if [[ "$SB_PHASE" != "down" ]]; then
				sb_now SB_BUILD_AT
			fi
			;;
		up)
			if [[ "$SB_PHASE" == "down" ]]; then
				sb_now now
				sb_duration SB_BUILD_FOR "$(( now - SB_BUILD_AT ))"
			elif [[ "$SB_PHASE" != "up" ]]; then
				SB_BUILD_FOR=""
			fi
			;;
	esac
	SB_PHASE="$phase"

	case "$phase" in
		bad) sgr="$SB_BAD_SGR" ;;
		up)
			sgr="$SB_UP_SGR"

			#only advertise the address once there is actually something behind it
			url="$SB_URL"

			if [ -n "$SB_BUILD_FOR" ]; then
				stamp="(built in $SB_BUILD_FOR)"
			fi
			;;
		*) sgr="$SB_DOWN_SGR" ;;
	esac

	#every field is padded to the width of the longest thing that can land in it, so
	#client and the url start at the same column no matter what the states say
	printf -v text ' server: %-8s   client: %-8s   %s%s ' "$server" "$client" "$url" \
		"${stamp:+    $stamp}"

	sb_size
	if (( ${#text} > SB_COLS )); then
		text="${text:0:SB_COLS}"
	else
		printf -v text '%-*s' "$SB_COLS" "$text"
	fi

	#everything from here goes on after the width maths, or the escapes would count as
	#visible columns and the bar would come out short.
	#
	#the bar is already bold, so \e[3m on top of it is bold italic, and \e[23m drops the
	#italic again without touching the bold
	if [ -n "$stamp" ]; then
		text="${text/"$stamp"/$'\e[3m'"$stamp"$'\e[23m'}"
	fi

	#a field that disagrees with the bar as a whole keeps its own colour, so half the pair
	#being up or still working reads at a glance regardless: a lone "ready" stays green on
	#a pink or red bar, a lone "building" stays pink on a red one. matched with the label
	#attached so a url containing either word can't be recoloured by accident, and $sgr
	#puts the bar's own colours back afterwards
	local field val hue

	for field in server client; do
		val="${!field}"
		hue=""

		if [[ "$val" == "ready" && "$phase" != "up" ]]; then
			hue="$SB_UP_SGR"
		elif [[ "$val" == "building" && "$phase" != "down" ]]; then
			hue="$SB_DOWN_SGR"
		fi

		if [ -n "$hue" ]; then
			text="${text/$field: $val/$field: ${hue}${val}${sgr}}"
		fi
	done

	#two forms of the same bar. SB_BAR lands on a row that is already blank, which is
	#the case directly after a scroll, and SB_PAINT clears first so it can repaint in
	#place: without that a bar that just got narrower would leave its old tail behind.
	#
	#the rest: hide the cursor so it isn't left blinking on top of the bar, bold, then
	#autowrap off so a bar built for a width the window no longer has is clipped at the
	#margin rather than spilling onto a second row, and finally park the cursor back at
	#the start of the bar
	SB_BAR=$'\e[?25l\e[?7l\e[m\e[1m'"$sgr$text"$'\e[m\r\e[?7h'
	SB_PAINT=$'\r\e[J'"$SB_BAR"
}

#pumps everything the dev servers print out to the terminal, lifting the bar out of the
#way for each line and setting it back down underneath. it also ticks on its own once a
#second, because the server can die and come back without saying a word about it
sb_loop()
{
	local line rc out midline=0 straddle=0 before="" after=""

	#drop the bar the instant ctrl+c lands instead of waiting for concurrently to finish
	#reaping its children, but don't exit: this loop has to stay alive to keep printing
	#their goodbyes, and concurrently is the one that should own the shutdown
	trap 'sb_close' EXIT INT TERM HUP

	#nothing to repair when the window changes size, just rebuild the bar at the new
	#width on the next pass instead of waiting out the rest of the second
	trap 'SB_LAST=-1' WINCH

	sb_paint

	while true; do
		line=""
		rc=0

		#equal unless a scan moves them apart, so a quiet pass never counts as a change
		before=""
		after=""

		IFS= read -r -t "$SB_TICK" line || rc=$?

		out=""
		straddle=$midline

		if [ -n "$line" ]; then
			SB_SHOWN=0

			if (( midline )); then
				#already part way through a line, so just carry on from where it stopped
				out="$line"
			else
				#wipe the bar off first, then print onto the cleared row. printing over
				#the top of it instead would be cheaper but it smears: a line only
				#overwrites the cells it actually prints, and a tab jumps the cursor
				#across cells without touching them, so the bar shows through the gaps
				out=$'\r\e[J'"$line"
			fi

			if (( rc == 0 )); then
				#\r\eD rather than \n. stdio is line buffered on a terminal, so a \n
				#anywhere in this string splits it into two writes, and the terminal is
				#free to draw a frame in the gap with the bar already gone. \eD moves
				#down and scrolls exactly like \n does, without tripping the flush
				out+=$'\r\eD'
				midline=0
			else
				#timed out or hit EOF mid-line, so don't invent a line break. the bar
				#stays down until the rest of it turns up, since painting now would
				#wipe the half of the line that has already been printed
				midline=1
			fi

			sb_state_id before
			sb_scan "$line"
			sb_state_id after
		fi

		#compose before any of this is printed. it forks, and forking halfway through a
		#repaint is a stall with the bar already wiped off the screen. painting doesn't
		#fork, so a noisy build only pays for the paint.
		#
		#three reasons to bother: the line just changed something, so say so immediately;
		#the read timed out, which is the poll coming round; or a second has gone by and
		#output is streaming hard enough that the poll never gets a look in
		if [[ "$before" != "$after" ]] || (( rc > 128 )) || (( SECONDS != SB_LAST )); then
			SB_LAST=$SECONDS
			sb_compose
		fi

		#the bar goes back after every single line, no exceptions. skipping it while
		#output is backed up sounds like a saving, but concurrently hands over whole
		#bursts at once, so the bar ends up off screen for the entire burst instead of
		#for one frame. that is what the flashing was
		if (( midline == 0 && SB_ON == 1 )); then
			SB_SHOWN=1

			#straight after a line the scroll has already left a blank row waiting,
			#so the bar can be set down without blanking anything first
			if [ -n "$line" ]; then
				out+="$SB_BAR"
			else
				out+="$SB_PAINT"
			fi
		fi

		#one write for the whole update, wrapped in synchronized output. mode 2026 tells
		#the terminal to hold its frame until the end marker, so it cannot draw the half
		#of this where the log line has landed on the bar and the bar hasn't been set
		#back down yet. terminals that don't know the mode ignore both markers.
		#
		#not when the write straddles a half read line though. a read that times out mid
		#line hands back whatever arrived, which can be the bare ESC of a colour code
		#with its "[47m" still in the pipe. a marker spliced into that seam is itself an
		#escape: the terminal takes the orphaned ESC as the start of the marker, eats it,
		#and the rest of the colour code turns up on screen as text. the markers are an
		#optimisation, the log is not, so the seam wins
		if [ -n "$out" ]; then
			if (( straddle == 0 && midline == 0 )); then
				printf '\e[?2026h%s\e[?2026l' "$out"
			else
				printf '%s' "$out"
			fi
		fi

		#read returns >128 on timeout, and anything else nonzero means the pipe closed
		if (( rc != 0 && rc <= 128 )); then
			break
		fi
	done
}
