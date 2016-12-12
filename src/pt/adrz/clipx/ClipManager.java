package pt.adrz.clipx;
import java.awt.Toolkit;
import java.awt.datatransfer.Clipboard;
import java.awt.datatransfer.ClipboardOwner;
import java.awt.datatransfer.DataFlavor;
import java.awt.datatransfer.FlavorEvent;
import java.awt.datatransfer.FlavorListener;
import java.awt.datatransfer.StringSelection;
import java.awt.datatransfer.Transferable;
import java.awt.datatransfer.UnsupportedFlavorException;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

import pt.adrz.clipx.gui.panels.Panels;

/**
 * Class that will caught clip board events, if it is a String, that 
 * String will be stored in a LinkedList
 * @author adriano
 *
 */
public class ClipManager implements FlavorListener, ClipboardOwner, EnableListener {
	private Clipboard clip;
	
	private Panels panels;
	
	private List<ClipboardListener> listeners = new ArrayList<ClipboardListener>();
	
	private ClipOptions opt = ClipOptions.getInstance();
	
	public ClipManager() {
		this.clip = Toolkit.getDefaultToolkit().getSystemClipboard();
		clip.setContents(clip.getContents(null), this);
		clip.addFlavorListener(this);
	}
	
	/**
	 * Constructor - Get and then Set the contents in clip board in order
	 * to get the ownership.
	 */
	public ClipManager(ClipboardListener listener) {
		this();
		this.listeners.add(listener);
	}
	
	public void setPanels(Panels panels) {
		this.panels = panels;
	}
	
	public synchronized void addClipboardListener(ClipboardListener listener) {
		this.listeners.add(listener);
	}

	public synchronized void removeClipboardListener(ClipboardListener listener) {
		this.listeners.remove(listener);
	}
	
	private synchronized void newString(String copyString) {
		Iterator<ClipboardListener> i = this.listeners.iterator();

		while (i.hasNext()) {
			i.next().newString(copyString);
		}
	}
	

	/**
	 * Replace the clip board content with the given string
	 * @param text String to be stored in the clip board
	 */
	public void setClipboard(String text) {
		// we don't need to be notified about this clip board change
		clip.removeFlavorListener(this);
		
		// add text to clip board ...
		Toolkit.getDefaultToolkit().getSystemClipboard().setContents(new StringSelection(text), this);
		
		// ... bring back the clip board notifications changes again
		clip.addFlavorListener(this);	
	}
	
	/**
	 * Clip board has new data
	 */
	@Override
	public void flavorsChanged(FlavorEvent e) {
		Transferable tf = null;
		
		try { 
			tf = clip.getContents(null); 
		}
		catch (IllegalStateException illStateEx) { 
			return;
		}
		
		String copyString = null;
		
		if ( tf.isDataFlavorSupported(DataFlavor.stringFlavor) && this.opt.isEnabled() ) {		

			try {

				copyString = (String)tf.getTransferData(DataFlavor.stringFlavor);

				if ( copyString != null ) {

					StringSelection ss = new StringSelection(copyString);
					Toolkit.getDefaultToolkit().getSystemClipboard().setContents(ss, this);
				}

				if ( !this.panels.getLeftPanel().getList().getModel().getItems().contains(copyString) ) {

					try {
						//listener.newString(copyString);
						this.newString(copyString);
					}
					catch (Exception ex) { 
						ex.printStackTrace(); 
					}
				}
			}
			catch (IOException ioEx) { }
			catch (UnsupportedFlavorException unFlvEx) { }
			catch (IllegalStateException illEx) { }
		}
	}
	
	@Override
	public void lostOwnership(Clipboard arg0, Transferable arg1) { 
		
	}

	@Override
	public void getClipboardOwnership() {
		clip.removeFlavorListener(this);
		this.clip = Toolkit.getDefaultToolkit().getSystemClipboard();
		clip.setContents(clip.getContents(null), this);
		clip.addFlavorListener(this);	
	}
}
