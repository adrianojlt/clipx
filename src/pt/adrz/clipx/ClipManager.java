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

/**
 * Class that will caught clipboard events, if it is a String, that 
 * String will be stored in a LinkedList
 * @author adriano
 *
 */
public class ClipManager implements FlavorListener, ClipboardOwner {

	/**
	 * SyS clipboard
	 */
	private Clipboard clip;
	
	/**
	 * Send notification to here ...
	 */
	private ClipboardListener listener;
	
	/**
	 * Constructor - Get and then Set the contents in clipboard in order
	 * to get the ownership.
	 */
	public ClipManager(ClipboardListener listener) {
		
		this.listener = listener;
		
		this.clip = Toolkit.getDefaultToolkit().getSystemClipboard();
		
		clip.setContents(clip.getContents(null), this);

		clip.addFlavorListener(this);
	}

	/**
	 * Replace the clipboard with the given string
	 * @param text String to be stored in the clipboard
	 */
	public void setClipboard(String text) {
		
		// we don't need to be notified about this change
		clip.removeFlavorListener(this);
		
		// add text to clipboard ...
		StringSelection ss = new StringSelection(text);
		Toolkit.getDefaultToolkit().getSystemClipboard().setContents(ss, this);
		
		// ... from now one we need clipboard changes notifications
		clip.addFlavorListener(this);	
	}
	
	
	/**
	 * Clipboard has new data
	 */
	@Override
	public void flavorsChanged(FlavorEvent e) {
		
		Transferable tf = null;
		
		try { tf = clip.getContents(null); }
		catch (IllegalStateException illStateEx) { return;}
		
		String copyString = null;
		
		if (tf.isDataFlavorSupported(DataFlavor.stringFlavor)) {		

			try {

				copyString = (String)tf.getTransferData(DataFlavor.stringFlavor);

				if (copyString != null) {

					StringSelection ss = new StringSelection(copyString);
					Toolkit.getDefaultToolkit().getSystemClipboard().setContents(ss, this);
				}

				if(!this.listener.getList().getModel().getItems().contains(copyString)) {

					try {
						listener.newString(copyString);
						//gui.getList().getModel().addElementTo(copyString, 0);
						//this.gui.getEditTA().setText(copyString);
					}
					catch (Exception ex) { ex.printStackTrace(); }
				}
				else {}
			}
			catch (IOException ioEx) { }
			catch (UnsupportedFlavorException unFlvEx) { }
			catch (IllegalStateException illEx) { }
		}
		else { }
	}
	
	
	@Override
	public void lostOwnership(Clipboard arg0, Transferable arg1) { }

}
