package pt.adrz.clipx;

import java.awt.MenuBar;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.ItemEvent;
import java.awt.event.ItemListener;

import javax.swing.JCheckBoxMenuItem;
import javax.swing.JMenu;
import javax.swing.JMenuBar;
import javax.swing.JMenuItem;

public class ClipMenuBar extends MenuBar implements ItemListener{
	
	private static final long serialVersionUID = 1L;

	private JMenuBar mainBar;

	private JMenu mFile, mEdit, mOptions, mAbout;

	private JMenuItem itemExit, itemPreferences;
	private JCheckBoxMenuItem itemActivate;
	
	private State state;

	
	public ClipMenuBar() {
		
		mainBar = new JMenuBar();

		mFile = new JMenu();
		mEdit = new JMenu();
		mOptions = new JMenu();
		mAbout = new JMenu();
		
		// Items
		itemExit = new JMenuItem("Exit");
		itemPreferences = new JMenuItem("Preferences");
		itemActivate = new JCheckBoxMenuItem("Activate");
		
		mFile.add(itemExit);
		mEdit.add(itemPreferences);
		mOptions.add(itemActivate);
		
		itemExit.addActionListener( new ActionListener() {
			@Override
			public void actionPerformed(ActionEvent arg0) {
				System.exit(0);
			}
		});
	}
	
	public void setState(State state) {
		this.state = state;
	}

	@Override
	public void itemStateChanged(ItemEvent e) {
		
	}
}
