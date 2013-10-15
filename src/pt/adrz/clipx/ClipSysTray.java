package pt.adrz.clipx;

import java.awt.AWTException;
import java.awt.CheckboxMenuItem;
import java.awt.Image;
import java.awt.MenuItem;
import java.awt.PopupMenu;
import java.awt.SystemTray;
import java.awt.TrayIcon;
import java.awt.TrayIcon.MessageType;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.net.URL;

import javax.swing.Icon;
import javax.swing.ImageIcon;
import javax.swing.JCheckBoxMenuItem;
import javax.swing.JPopupMenu;

public class ClipSysTray implements ActionListener, MouseListener {
	
	private static final String MENU_ITEM_ITEM01 	= "Item01";
	private static final String MENU_ITEM_ITEM02 	= "Item02";
	private static final String MENU_ITEM_ITEM03 	= "Item03";
	private static final String MENU_ITEM_ACTIVATE 	= "Active";
	private static final String MENU_ITEM_ABOUT 	= "About";
	private static final String MENU_ITEM_EXIT 		= "Exit";
	private static final String TOOL_TIP 			= "ClipX";
	private static final String DISPLAY_MESSAGE 	= "clipboard Strings will be Saved ...";
	private static final String ICON_IMAGE_PATH 	= "mainIcon.gif";
	
	private SystemTray tray;
	private PopupMenu mainMenu;
	private TrayIcon icon;
	
	private JPopupMenu jPopUpM = new JPopupMenu();
	private CheckboxMenuItem mItemCheck;

	private MenuItem mItem01; 
	private MenuItem mItem02; 
	private MenuItem mItem03; 
	private MenuItem mItemActivate; 
	private MenuItem mItemAbout; 
	private MenuItem mItemExit; 
	
	private ClipGUI gui;
	
	public ClipSysTray(final ClipGUI gui) {
		
		this.gui = gui;
		
		this.tray = SystemTray.getSystemTray();
		
		this.iniMenu();
		
		this.iniTrayIcon();
	}
	
	private void iniMenu() {
		
		this.mItem01 = new CheckboxMenuItem(MENU_ITEM_ITEM01);
		this.mItemCheck = new CheckboxMenuItem(MENU_ITEM_ACTIVATE, true);
		this.mItem02 = new CheckboxMenuItem(MENU_ITEM_ITEM02);
		this.mItem03 = new CheckboxMenuItem(MENU_ITEM_ITEM03);
		this.mItemActivate = new CheckboxMenuItem(MENU_ITEM_ACTIVATE);
		this.mItemAbout = new MenuItem(MENU_ITEM_ABOUT);
		this.mItemExit = new MenuItem(MENU_ITEM_EXIT);
		
		//this.mItem01.setShortcut(swt)
		
		this.mItem01.addActionListener(this);
		this.mItem02.addActionListener(this);
		this.mItem03.addActionListener(this);
		this.mItemActivate.addActionListener(this);
		this.mItemAbout.addActionListener(this);
		this.mItemExit.addActionListener(this);
		
		this.mainMenu = new PopupMenu();
		this.mainMenu.add(mItemCheck);
		this.mainMenu.add(mItem01);
		this.mainMenu.add(mItem02);
		this.mainMenu.add(mItem03);
		this.mainMenu.add(mItemActivate);
		this.mainMenu.add(mItemAbout);
		this.mainMenu.add(mItemExit);
		this.mainMenu.addActionListener(this);
	}
	
	private void iniTrayIcon() {
		
		URL imgURL =  getClass().getResource(ICON_IMAGE_PATH);
		Image image = new ImageIcon(imgURL, "ClipX").getImage();

		icon = new TrayIcon(image);

		icon.setPopupMenu(this.mainMenu);
		icon.setImageAutoSize(true);
		icon.addActionListener(this);
		
		icon.setToolTip(TOOL_TIP);
		
		try { tray.add(icon); } catch (AWTException eAWT) { }
		
		icon.displayMessage(TOOL_TIP, DISPLAY_MESSAGE, MessageType.INFO);
		
		icon.addMouseListener(this);
	}

	@Override
	public void actionPerformed(ActionEvent e) {
		
		try { if (e.getActionCommand().equals(MENU_ITEM_EXIT)) { System.exit(0); }	}
		catch (NullPointerException eNULL) { }
		catch (Exception eEX) { }	
		
		
	}

	@Override
	public void mouseClicked(MouseEvent e) {
		
		if (e.getButton() == MouseEvent.BUTTON1) {
			if (this.gui.isVisible())
				this.gui.setVisible(false);
			else
				this.gui.setVisible(true);
		}
	}

	@Override
	public void mouseEntered(MouseEvent arg0) { }

	@Override
	public void mouseExited(MouseEvent arg0) { }

	@Override
	public void mousePressed(MouseEvent arg0) { }

	@Override
	public void mouseReleased(MouseEvent arg0) { }
}
