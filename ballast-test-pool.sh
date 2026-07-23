#!/usr/bin/env bash
#
# ballast-test-pool.sh - Create/destroy a loopback-backed ZFS test pool
#
# Usage:
#   sudo ./ballast-test-pool.sh up      # create pool
#   sudo ./ballast-test-pool.sh down    # destroy pool and clean up
#   sudo ./ballast-test-pool.sh status  # show pool/loop device state
#
set -euo pipefail

POOL_NAME="ballast-test"
WORK_DIR="/var/tmp/ballast-test-pool"
DISK_COUNT=4
DISK_SIZE="1G"
VDEV_TYPE="mirror" # "" for stripe, "mirror", or "raidz"
STATE_FILE="${WORK_DIR}/.loop_devices"

require_root() {
  if [[ $EUID -ne 0 ]]; then
    echo "Error: this script must be run as root (sudo)." >&2
    exit 1
  fi
}

require_zfs() {
  command -v zpool >/dev/null 2>&1 || {
    echo "Error: zpool not found. Is ZFS installed?" >&2
    exit 1
  }
}

pool_exists() {
  zpool list -H -o name 2>/dev/null | grep -qx "${POOL_NAME}"
}

cmd_up() {
  if pool_exists; then
    echo "Pool '${POOL_NAME}' already exists. Run '$0 down' first." >&2
    exit 1
  fi

  mkdir -p "${WORK_DIR}"
  : >"${STATE_FILE}"

  echo "Creating ${DISK_COUNT} backing file(s) of ${DISK_SIZE} in ${WORK_DIR}..."
  local loop_devices=()
  for i in $(seq 1 "${DISK_COUNT}"); do
    local img="${WORK_DIR}/disk${i}.img"
    truncate -s "${DISK_SIZE}" "${img}"

    local loop_dev
    loop_dev=$(losetup -f --show "${img}")
    loop_devices+=("${loop_dev}")
    echo "${loop_dev}" >>"${STATE_FILE}"
    echo "  ${img} -> ${loop_dev}"
  done

  echo "Creating pool '${POOL_NAME}' (${VDEV_TYPE:-stripe})..."
  zpool create -f "${POOL_NAME}" ${VDEV_TYPE} "${loop_devices[@]}"

  echo "Done."
  zpool status "${POOL_NAME}"
}

cmd_down() {
  if pool_exists; then
    echo "Destroying pool '${POOL_NAME}'..."
    zpool destroy "${POOL_NAME}"
  else
    echo "Pool '${POOL_NAME}' not found, skipping destroy."
  fi

  if [[ -f "${STATE_FILE}" ]]; then
    echo "Detaching loop devices..."
    while IFS= read -r loop_dev; do
      [[ -b "${loop_dev}" ]] && losetup -d "${loop_dev}" 2>/dev/null &&
        echo "  detached ${loop_dev}" ||
        echo "  ${loop_dev} already detached, skipping"
    done <"${STATE_FILE}"
  fi

  echo "Removing ${WORK_DIR}..."
  rm -rf "${WORK_DIR}"
  echo "Done."
}

cmd_status() {
  if pool_exists; then
    zpool status "${POOL_NAME}"
  else
    echo "Pool '${POOL_NAME}' does not exist."
  fi

  if [[ -f "${STATE_FILE}" ]]; then
    echo
    echo "Loop devices:"
    while IFS= read -r loop_dev; do
      losetup "${loop_dev}" 2>/dev/null || echo "  ${loop_dev} (not attached)"
    done <"${STATE_FILE}"
  fi
}

require_root
require_zfs

case "${1:-}" in
  up) cmd_up ;;
  down) cmd_down ;;
  status) cmd_status ;;
  *)
    echo "Usage: $0 {up|down|status}" >&2
    exit 1
    ;;
esac
