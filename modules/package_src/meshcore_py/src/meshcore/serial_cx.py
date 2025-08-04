""" 
    mccli.py : CLI interface to MeschCore BLE companion app
"""
import asyncio
import logging
import serial_asyncio
import time

# Get logger
logger = logging.getLogger("meshcore")

class SerialConnection:
    def __init__(self, port, baudrate, cx_dly=0.5):
        self.port = port
        self.baudrate = baudrate
        self.frame_started = False
        self.frame_size = 0
        self.transport = None
        self.header = b""
        self.reader = None
        self.inframe = b""
        self._disconnect_callback = None
        self.cx_dly = cx_dly
        self.MAX_FRAME_SIZE = 4096  # Maximum allowed frame size
        self.TX_SYNC_BYTE = 0x3c  # Frame start byte for sending
        self.RX_SYNC_BYTE = 0x3e  # Frame start byte for receiving
        self.frame_start_time = None  # Track when frame started
        self.FRAME_TIMEOUT = 5.0  # Timeout in seconds for incomplete frames
        self._timeout_task = None  # Task for checking timeouts
        self.last_rx_time = None  # Track last successful receive
        self.CONNECTION_TIMEOUT = 30.0  # Consider connection dead after this many seconds of no data

    class MCSerialClientProtocol(asyncio.Protocol):
        def __init__(self, cx):
            self.cx = cx

        def connection_made(self, transport):
            self.cx.transport = transport
            logger.debug('port opened')
            if isinstance(transport, serial_asyncio.SerialTransport) and transport.serial:
                transport.serial.rts = False  # You can manipulate Serial object via transport
                # Also set DTR to ensure proper state
                transport.serial.dtr = True
    
        def data_received(self, data):
            self.cx.handle_rx(data)    
    
        def connection_lost(self, exc):
            logger.debug('Serial port closed')
            if self.cx._disconnect_callback:
                asyncio.create_task(self.cx._disconnect_callback("serial_disconnect"))
    
        def pause_writing(self):
            logger.debug('pause writing')
    
        def resume_writing(self):
            logger.debug('resume writing')

    async def connect(self):
        """
        Connects to the device
        """
        loop = asyncio.get_running_loop()
        await serial_asyncio.create_serial_connection(
                loop, lambda: self.MCSerialClientProtocol(self), 
                self.port, baudrate=self.baudrate)

        # Try to reset the device by toggling DTR
        if self.transport and hasattr(self.transport, 'serial'):
            try:
                logger.debug("Toggling DTR to reset device")
                self.transport.serial.dtr = False
                await asyncio.sleep(0.1)
                self.transport.serial.dtr = True
                await asyncio.sleep(0.1)
            except Exception as e:
                logger.warning(f"Failed to toggle DTR: {e}")
        
        await asyncio.sleep(self.cx_dly) # wait for cx to establish
        
        # Flush any stale data in buffers
        self.flush_buffers()
        
        logger.info("Serial Connection started")
        
        # Start timeout monitoring task
        self._timeout_task = asyncio.create_task(self._monitor_frame_timeout())
        
        return self.port

    def set_reader(self, reader) :
        self.reader = reader

    def handle_rx(self, data: bytearray):
        """Handle incoming serial data with improved error handling and sync recovery"""
        if not data:
            return
            
        # If we're not synchronized, look for sync byte
        if not self.frame_started and len(self.header) == 0:
            sync_pos = data.find(self.RX_SYNC_BYTE)
            if sync_pos == -1:
                # No sync byte found, discard all data
                logger.debug(f"No sync byte found in {len(data)} bytes, discarding")
                return
            elif sync_pos > 0:
                # Discard data before sync byte
                logger.warning(f"Discarding {sync_pos} bytes before sync byte")
                data = data[sync_pos:]
        
        headerlen = len(self.header)
        framelen = len(self.inframe)
        
        if not self.frame_started:  # Waiting for frame header
            bytes_needed = 3 - headerlen
            if len(data) >= bytes_needed:
                # Complete the header
                self.header = self.header + data[:bytes_needed]
                
                # Validate sync byte
                if self.header[0] != self.RX_SYNC_BYTE:
                    logger.error(f"Invalid sync byte: {self.header[0]:02x}, resetting")
                    self.reset_frame_state()
                    self.handle_rx(data[1:])  # Try to resync from next byte
                    return
                
                # Extract and validate frame size
                self.frame_size = int.from_bytes(self.header[1:3], byteorder='little')
                
                if self.frame_size > self.MAX_FRAME_SIZE:
                    logger.error(f"Frame size {self.frame_size} exceeds maximum {self.MAX_FRAME_SIZE}, resetting")
                    self.reset_frame_state()
                    self.handle_rx(data[1:])  # Try to resync
                    return
                
                if self.frame_size == 0:
                    logger.warning("Received frame with size 0, ignoring")
                    self.reset_frame_state()
                    self.handle_rx(data[bytes_needed:])
                    return
                
                self.frame_started = True
                self.frame_start_time = time.time()  # Track when frame started
                # Process remaining data
                if len(data) > bytes_needed:
                    self.handle_rx(data[bytes_needed:])
            else:
                # Accumulate partial header
                self.header = self.header + data
        else:
            # Reading frame payload
            bytes_needed = self.frame_size - framelen
            if len(data) >= bytes_needed:
                # Complete the frame
                self.inframe = self.inframe + data[:bytes_needed]
                
                # Dispatch the complete frame
                if self.reader is not None and len(self.inframe) > 0:
                    asyncio.create_task(self.reader.handle_rx(self.inframe))
                    self.last_rx_time = time.time()  # Update last receive time
                
                # Reset for next frame
                self.reset_frame_state()
                
                # Process remaining data
                if len(data) > bytes_needed:
                    self.handle_rx(data[bytes_needed:])
            else:
                # Accumulate partial frame
                self.inframe = self.inframe + data
    
    def reset_frame_state(self):
        """Reset the frame parser state"""
        self.frame_started = False
        self.frame_size = 0
        self.header = b""
        self.inframe = b""
        self.frame_start_time = None

    async def send(self, data):
        if not self.transport:
            logger.error("Transport not connected, cannot send data")
            return
        size = len(data)
        pkt = bytes([self.TX_SYNC_BYTE]) + size.to_bytes(2, byteorder="little") + data
        logger.debug(f"sending pkt : {pkt}")
        self.transport.write(pkt)
        
    async def _monitor_frame_timeout(self):
        """Monitor for frame timeouts and connection health"""
        try:
            while self.transport:
                await asyncio.sleep(1.0)  # Check every second
                
                # Check frame timeout
                if self.frame_started and self.frame_start_time:
                    elapsed = time.time() - self.frame_start_time
                    if elapsed > self.FRAME_TIMEOUT:
                        logger.error(f"Frame timeout after {elapsed:.1f}s, resetting parser")
                        self.reset_frame_state()
                
                # Check connection health
                if self.last_rx_time:
                    elapsed = time.time() - self.last_rx_time
                    if elapsed > self.CONNECTION_TIMEOUT:
                        logger.warning(f"No data received for {elapsed:.1f}s, connection may be dead")
                        # Could trigger reconnect or alert here
                        
        except asyncio.CancelledError:
            logger.debug("Frame timeout monitor cancelled")
    
    async def disconnect(self):
        """Close the serial connection."""
        # Cancel timeout monitor
        if self._timeout_task:
            self._timeout_task.cancel()
            try:
                await self._timeout_task
            except asyncio.CancelledError:
                pass
        
        if self.transport:
            self.transport.close()
            self.transport = None
            logger.debug("Serial Connection closed")
            
    def flush_buffers(self):
        """Flush serial input/output buffers"""
        if self.transport and hasattr(self.transport, 'serial'):
            try:
                self.transport.serial.reset_input_buffer()
                self.transport.serial.reset_output_buffer()
                logger.debug("Flushed serial buffers")
            except Exception as e:
                logger.error(f"Failed to flush buffers: {e}")
    
    async def recover_from_error(self):
        """Attempt to recover from communication errors"""
        logger.warning("Attempting to recover from serial communication error")
        
        # Reset frame parser state
        self.reset_frame_state()
        
        # Flush buffers
        self.flush_buffers()
        
        # Small delay to let things settle
        await asyncio.sleep(0.1)
        
        logger.info("Recovery attempt completed")
    
    def set_disconnect_callback(self, callback):
        """Set callback to handle disconnections."""
        self._disconnect_callback = callback
