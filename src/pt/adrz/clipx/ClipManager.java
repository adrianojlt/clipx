package pt.adrz.clipx;
import java.awt.AWTException;
import java.awt.MenuItem;
import java.awt.PopupMenu;
import java.awt.SystemTray;
import java.awt.Toolkit;
import java.awt.TrayIcon;
import java.awt.TrayIcon.MessageType;
import java.awt.datatransfer.Clipboard;
import java.awt.datatransfer.ClipboardOwner;
import java.awt.datatransfer.DataFlavor;
import java.awt.datatransfer.FlavorEvent;
import java.awt.datatransfer.FlavorListener;
import java.awt.datatransfer.StringSelection;
import java.awt.datatransfer.Transferable;
import java.awt.datatransfer.UnsupportedFlavorException;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.KeyEvent;
import java.awt.event.KeyListener;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.io.IOException;
import javax.swing.ImageIcon;

/**
 * Class that will caught clipboard events, if it is a String, that 
 * String will be stored in a LinkedList
 * @author adriano
 *
 */
public class ClipManager implements FlavorListener, ClipboardOwner, ActionListener, MouseListener, KeyListener{

	private static final String MENU_ITEM_ABOUT 	= "About";
	private static final String MENU_ITEM_EXIT 		= "Exit";
	private static final String TOOL_TIP 			= "ClipX";
	
	/**
	 * SyS clipboard
	 */
	private Clipboard clip;
	
	
	/**
	 * GUI of the application
	 */
	private ClipGUI gui;
	
	// SysTray vars
	private PopupMenu	popupMenu;
	private TrayIcon 	trayIcon;
	private SystemTray 	sysTray;
	
	// SysTray menu items 
	MenuItem item01;
	MenuItem item02;
	MenuItem item03;
	MenuItem mItemAbout;
	MenuItem mItemExit;
	
	
	
	
	
	/**
	 * Constructor - Get and then Set the contents in clipboard in order
	 * to get the ownership.
	 */
	public ClipManager() {
		
		this.clip = Toolkit.getDefaultToolkit().getSystemClipboard();
		
		this.sysTray = SystemTray.getSystemTray();
		
		clip.setContents(clip.getContents(null), this);
		clip.addFlavorListener(this);
		
		gui = new ClipGUI(this);
		
		this.iniSysTray();
		
		this.gui.addKeyListener(this);
	}

	
	
	
	/**
	 * create system tray icon resources
	 */
	private void iniSysTray() {
		
		item01 		= new MenuItem("item01");
		item02 		= new MenuItem("item02");
		item03 		= new MenuItem("item03");
		mItemAbout 	= new MenuItem(MENU_ITEM_ABOUT);
		
		mItemAbout.setEnabled(false);
		
		mItemExit	= new MenuItem(MENU_ITEM_EXIT);
		
		item01.addActionListener(this);
		item02.addActionListener(this);
		item03.addActionListener(this);
		mItemAbout.addActionListener(this);
		mItemExit.addActionListener(this);
		
		popupMenu = new PopupMenu();
		popupMenu.add(item01);
		popupMenu.add(item02);
		popupMenu.add(item03);
		popupMenu.add(mItemAbout);
		popupMenu.add(mItemExit);
		popupMenu.addActionListener(this);		
			
		trayIcon = new TrayIcon(new ImageIcon("img/mainIcon.gif", "ClipX").getImage());
		trayIcon.setPopupMenu(popupMenu);
		trayIcon.setImageAutoSize(true);
		trayIcon.addActionListener(this);
		trayIcon.addMouseListener(this);
		
		trayIcon.setToolTip(TOOL_TIP);
		
		try { sysTray.add(trayIcon); }
		catch (AWTException eAWT) { }
		
		trayIcon.displayMessage("ClipX", "clipboard Strings will be Saved ...", MessageType.INFO);
	}
	
	
	
	
	/**
	 * Clipboard has new data
	 */
	@Override
	public void flavorsChanged(FlavorEvent e) {
		
		//clip.removeFlavorListener(this);
		
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
				if(!this.gui.getList().getModel().getItems().contains(copyString)) {
					try {
						gui.getList().getModel().addElementTo(copyString, 0);
						this.gui.getEditTA().setText(copyString);
					}
					catch (Exception ex) {
						ex.printStackTrace();
					}
				}
				else {}
			}
			catch (IOException ioEx) { 
				System.out.println("ioex");
			}
			catch (UnsupportedFlavorException unFlvEx) {
				System.out.println("unFlvEx");
			}
			catch (IllegalStateException illEx) {
				System.out.println("illEx");
			}
		}
		else { }
		
		//clip.addFlavorListener(this);
		
		//try { Thread.sleep(1000); } 
		//catch (InterruptedException ex) { ex.printStackTrace(); }
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
	
	
	
	
	
	
	
	
	
	@Override
	public void lostOwnership(Clipboard arg0, Transferable arg1) {
	}



	/**
	 * Every event from systray menu is processed here
	 */
	@Override
	public void actionPerformed(ActionEvent e) {
		try {
			if (e.getActionCommand().equals(MENU_ITEM_EXIT)) { System.exit(0); }	
		}
		catch (NullPointerException eNULL) { }
		catch (Exception eEX) { }	
	}



	/**
	 * 
	 * Process the event when a single left mouse click is done over systray ClipX icon
	 */
	@Override
	public void mouseClicked(MouseEvent e) {
		if (e.getButton() == MouseEvent.BUTTON1) { gui.setVisible(true); }
	}



	
	

	@Override
	public void mouseEntered(MouseEvent e) {
		
		
	}




	@Override
	public void mouseExited(MouseEvent e) {
		
		
	}




	@Override
	public void mousePressed(MouseEvent e) {
		
	}




	@Override
	public void mouseReleased(MouseEvent arg0) {
		
		
	}




	@Override
	public void keyPressed(KeyEvent arg0) {
		// TODO Auto-generated method stub
		System.out.println("keypressed = "+ arg0.getKeyCode());
	}




	@Override
	public void keyReleased(KeyEvent arg0) {
		// TODO Auto-generated method stub
		
	}




	@Override
	public void keyTyped(KeyEvent arg0) {
		// TODO Auto-generated method stub
		
	}




	



	

	
}
